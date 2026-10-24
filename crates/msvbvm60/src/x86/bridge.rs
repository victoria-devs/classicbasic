// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! MSVBVM60 32-bit x86 IAT Ordinal Bridge & Variant Memory Extensions.
//!
//! Bridges the hardware-level `cb_x86::X86Cpu` and `cb_x86::X86Memory` to the 64-bit
//! `msvbvm60` runtime exports, COM objects, and `Variant` / `SafeArray` marshaling.

use cb_x86::{HookCallback, X86Cpu, X86Error, X86Memory};

/// Registers the `msvbvm60` IAT hook dispatcher (`0xDEAD0000 | ordinal`) with `cb-x86`.
pub fn register_msvbvm_iat_dispatcher() {
    cb_x86::set_iat_dispatcher(|cpu, hook_id| cpu.dispatch_msvbvm_iat_hook(hook_id));
}

/// Extension trait adding `Variant` memory marshaling to `cb_x86::X86Memory`.
pub trait X86MemoryVbExt {
    fn read_variant(&self, addr: u32) -> crate::types::Variant;
    fn read_variant_deref(&self, addr: u32) -> crate::types::Variant;
    fn write_variant(&mut self, addr: u32, var: &crate::types::Variant);
}

impl X86MemoryVbExt for X86Memory {
    /// Reads a 16-byte Variant from emulator memory at `addr`.
    fn read_variant(&self, addr: u32) -> crate::types::Variant {
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
    fn read_variant_deref(&self, addr: u32) -> crate::types::Variant {
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
    fn write_variant(&mut self, addr: u32, var: &crate::types::Variant) {
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
}

/// Extension trait adding MSVBVM60 ordinal dispatch and COM/Variant helpers to `cb_x86::X86Cpu`.
pub trait X86CpuVbExt {
    fn dispatch_msvbvm_iat_hook(&mut self, hook_id: u32) -> Result<(), X86Error>;
    fn register_host_com_object(&mut self, host_ptr: usize) -> u32;
    fn read_variant_for_host(&self, addr: u32) -> crate::types::Variant;
    fn write_host_variant(&mut self, addr: u32, var: &crate::types::Variant);
    fn variant_to_string(&self, var: &crate::types::Variant) -> String;
    fn variant_to_f64(&self, v: &crate::types::Variant) -> Option<f64>;
    fn compare_variants(
        &self,
        v1: &crate::types::Variant,
        v2: &crate::types::Variant,
        text_compare: bool,
    ) -> std::cmp::Ordering;
}

impl X86CpuVbExt for X86Cpu {
    /// Dispatches an IAT hook call (0xDEAD0000 | ordinal) intercepted from PE indirect call.
    fn dispatch_msvbvm_iat_hook(&mut self, hook_id: u32) -> Result<(), X86Error> {
        if let Some(callback) = self.hooks.get(&hook_id) {
            let cb = callback as *const HookCallback;
            return unsafe { (*cb)(self) };
        }

        let ret_addr = self.pop32();
        self.pending_hook_ret = Some(ret_addr);
        let ordinal = (hook_id & 0xFFFF) as u16;

        match ordinal {
            0x8001 => {
                // WriteFile(hFile, lpBuffer, nBytes, lpWritten, lpOverlapped)
                let h_file = self.mem.read_u32(self.regs.esp) as usize;
                let lp_buf = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let n_bytes = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let lp_written = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                self.regs.esp = self.regs.esp.wrapping_add(20);

                let mut buf = vec![0u8; n_bytes as usize];
                for i in 0..n_bytes {
                    buf[i as usize] = self.mem.read_u8(lp_buf.wrapping_add(i));
                }
                let mut written: u32 = 0;
                let res = unsafe {
                    crate::pe::shims::win32_WriteFile(
                        h_file,
                        buf.as_ptr(),
                        n_bytes,
                        &mut written,
                        core::ptr::null_mut(),
                    )
                };
                if lp_written != 0 {
                    self.mem.write_u32(lp_written, written);
                }
                self.regs.eax = res as u32;
            }
            0x8002 => {
                // GetStdHandle(nStdHandle)
                let n_handle = self.mem.read_u32(self.regs.esp) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let res = unsafe { crate::pe::shims::win32_GetStdHandle(n_handle) };
                self.regs.eax = res as u32;
            }
            0x8003 => {
                // ExitProcess(uExitCode)
                let exit_code = self.mem.read_u32(self.regs.esp) as i32;
                self.exit_code = exit_code;
                self.is_halted = true;
                return Ok(());
            }
            187 => {
                // DllFunctionCall(p_info)
                let p_info = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if p_info != 0 {
                    let dll_ptr = self.mem.read_u32(p_info);
                    let fn_ptr = self.mem.read_u32(p_info.wrapping_add(4));
                    let cache_ptr = self.mem.read_u32(p_info.wrapping_add(12));
                    let dll_name = String::from_utf8_lossy(&self.mem.read_ansi(dll_ptr)).to_string();
                    let ord = if fn_ptr == 0 {
                        self.mem.read_u16(p_info.wrapping_add(8)) as u32
                    } else if (fn_ptr & 0xFFFF0000) == 0 {
                        fn_ptr
                    } else {
                        let fn_name = String::from_utf8_lossy(&self.mem.read_ansi(fn_ptr)).to_string();
                        crate::pe::resolver::resolve_symbol_ordinal(&dll_name, Some(&fn_name), None) as u32
                    };
                    let hook = 0xDEAD0000 | ord;
                    if cache_ptr != 0 {
                        self.mem.write_u32(cache_ptr, hook);
                    }
                    self.regs.eax = hook;
                } else {
                    self.regs.eax = 0;
                }
            }
            186 => {
                // ProcCallEngine
                let res = unsafe { crate::thun_rt::execute_pcode_proc(self.regs.edx as usize) };
                self.regs.eax = res as u32;
            }
            199 => {
                // MethCallEngine
                let res = unsafe { crate::thun_rt::execute_pcode_proc(self.regs.edx as usize) };
                self.regs.eax = res as u32;
            }
            100 => {
                // ThunRTMain(lp_vb_header)
                let lp_vb_header = self.mem.read_u32(self.regs.esp) as *const crate::types::VBHeader;
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let res = unsafe { crate::thun_rt::ThunRTMain(lp_vb_header) };
                self.regs.eax = res as u32;
            }
            138 => {
                // __vbaStrCopy(dst: ecx, src: edx) -> eax
                let dst = self.regs.ecx;
                let src = self.regs.edx;
                let res = if src == 0 {
                    0
                } else {
                    let chars = self.mem.read_bstr(src);
                    self.mem.alloc_bstr(&chars)
                };
                if dst != 0 {
                    self.mem.write_u32(dst, res);
                }
                self.regs.eax = res;
            }
            139 => {
                // __vbaStrMove(dst: ecx = *mut Bstr, src: edx = Bstr) -> eax = Bstr
                let dst = self.regs.ecx;
                let src = self.regs.edx;
                if dst != 0 {
                    self.mem.write_u32(dst, src);
                }
                self.regs.eax = src;
            }
            405 => {
                // __vbaStrCat(s1, s2): push s1; push s2; -> [esp] = s2, [esp + 4] = s1
                let s2 = self.mem.read_u32(self.regs.esp);
                let s1 = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let mut chars = self.mem.read_bstr(s1);
                chars.extend(self.mem.read_bstr(s2));
                let res = self.mem.alloc_bstr(&chars);
                self.regs.eax = res;
            }
            426 => {
                // __vbaStrToAnsi(dst: *mut *mut u8, unicode_src: Bstr) -> *mut u8
                let dst = self.mem.read_u32(self.regs.esp);
                let src = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let chars = self.mem.read_bstr(src);
                let ansi_bytes: Vec<u8> = chars.into_iter().map(|c| (c & 0xFF) as u8).collect();
                let res = self.mem.alloc_ansi(&ansi_bytes);
                if dst != 0 {
                    self.mem.write_u32(dst, res);
                }
                self.regs.eax = res;
            }
            427 => {
                // __vbaStrToUnicode(dst: *mut Bstr, ansi_src: *const u8) -> Bstr
                let dst = self.mem.read_u32(self.regs.esp);
                let src = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let bytes = self.mem.read_ansi(src);
                let chars: Vec<u16> = bytes.into_iter().map(|b| b as u16).collect();
                let res = self.mem.alloc_bstr(&chars);
                if dst != 0 {
                    self.mem.write_u32(dst, res);
                }
                self.regs.eax = res;
            }
            327 => {
                // __vbaLenBstr(s: Bstr) -> i32 (character count)
                let s = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = if s == 0 {
                    0
                } else {
                    self.mem.read_u32(s.wrapping_sub(4)) / 2
                };
            }
            328 => {
                // __vbaLenBstrB(s: Bstr) -> i32 (byte count)
                let s = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = if s == 0 {
                    0
                } else {
                    self.mem.read_u32(s.wrapping_sub(4))
                };
            }
            329 => {
                // __vbaLenVar(dst: *mut Variant, src: *const Variant) -> dst in EAX (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let var = self.mem.read_variant_deref(src_ptr);
                let s = self.variant_to_string(&var);
                let len = s.encode_utf16().count() as i32;
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_I4;
                out_var.data[0..4].copy_from_slice(&(len as u32).to_le_bytes());
                if dst_ptr != 0 {
                    self.mem.write_variant(dst_ptr, &out_var);
                }
                self.regs.eax = dst_ptr;
            }
            330 => {
                // __vbaLenVarB(dst: *mut Variant, src: *const Variant) -> dst in EAX (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let var = self.mem.read_variant_deref(src_ptr);
                let s = self.variant_to_string(&var);
                let len = (s.encode_utf16().count() * 2) as i32;
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_I4;
                out_var.data[0..4].copy_from_slice(&(len as u32).to_le_bytes());
                if dst_ptr != 0 {
                    self.mem.write_variant(dst_ptr, &out_var);
                }
                self.regs.eax = dst_ptr;
            }
            417 => {
                // __vbaStrI2(val: i16) -> Bstr
                let val = self.mem.read_u32(self.regs.esp) as i16;
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let s = format!("{}", val);
                self.regs.eax = self.mem.alloc_bstr_from_str(&s);
            }
            418 => {
                // __vbaStrI4(val: i32) -> Bstr
                let val = self.mem.read_i32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let s = format!("{}", val);
                self.regs.eax = self.mem.alloc_bstr_from_str(&s);
            }
            425 => {
                // __vbaStrUI1(val: u8) -> Bstr
                let val = self.mem.read_u32(self.regs.esp) as u8;
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let s = format!("{}", val);
                self.regs.eax = self.mem.alloc_bstr_from_str(&s);
            }
            273 => {
                // __vbaI4Var(v: *const Variant) -> i32 (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let res = var.as_i32().unwrap_or(0) as u32;
                self.regs.eax = res;
            }
            452 => {
                // __vbaVarCat(dst, right, left) -> *mut Variant (pops 12 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let right_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let left_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let vl = self.mem.read_variant_deref(left_ptr);
                let vr = self.mem.read_variant_deref(right_ptr);
                let sl = self.variant_to_string(&vl);
                let sr = self.variant_to_string(&vr);
                let combined = format!("{}{}", sl, sr);
                let new_bstr = self.mem.alloc_bstr_from_str(&combined);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            462 => {
                // __vbaVarErrI4(dst: *mut Variant, err_code: i32) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let err_code = self.mem.read_i32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_ERROR;
                out_var.data[0..4].copy_from_slice(&(err_code as u32).to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            407 => {
                // __vbaStrComp(comp, s1, s2) -> i32 (pops 12 bytes)
                // push s2; push s1; push comp; call __vbaStrComp
                let comp = self.mem.read_i32(self.regs.esp);
                let s2_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let s1_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let s1 = self.mem.read_bstr_string(s1_ptr);
                let s2 = self.mem.read_bstr_string(s2_ptr);
                let res = if comp == 1 {
                    s1.to_lowercase().cmp(&s2.to_lowercase())
                } else {
                    s1.cmp(&s2)
                };
                let r: i32 = match res {
                    core::cmp::Ordering::Less => -1,
                    core::cmp::Ordering::Equal => 0,
                    core::cmp::Ordering::Greater => 1,
                };
                self.regs.eax = r as u32;
            }
            279 => {
                // __vbaInStrVar(dst, comp, needle, haystack, start) -> dst in EAX (pops 20 bytes)
                // push start; push haystack; push needle; push comp; push dst; call __vbaInStrVar
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let comp = self.mem.read_i32(self.regs.esp.wrapping_add(4));
                let needle_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let hay_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                let start = self.mem.read_i32(self.regs.esp.wrapping_add(16));
                self.regs.esp = self.regs.esp.wrapping_add(20);
                let needle_var = if needle_ptr != 0 { self.mem.read_variant_deref(needle_ptr) } else { crate::types::Variant::empty() };
                let hay_var = if hay_ptr != 0 { self.mem.read_variant_deref(hay_ptr) } else { crate::types::Variant::empty() };
                let start_pos = start.max(1);
                let s1 = self.variant_to_string(&hay_var);
                let s2 = self.variant_to_string(&needle_var);
                let idx = if s2.is_empty() {
                    start_pos
                } else {
                    let u1: Vec<u16> = s1.encode_utf16().collect();
                    let u2: Vec<u16> = s2.encode_utf16().collect();
                    let start_idx = (start_pos - 1) as usize;
                    if start_idx >= u1.len() {
                        0
                    } else if comp == 1 {
                        let s1_lower: Vec<char> = s1.to_lowercase().chars().collect();
                        let s2_lower: Vec<char> = s2.to_lowercase().chars().collect();
                        if start_idx >= s1_lower.len() {
                            0
                        } else {
                            s1_lower[start_idx..]
                                .windows(s2_lower.len())
                                .position(|window| window == s2_lower.as_slice())
                                .map(|pos| (start_idx + pos + 1) as i32)
                                .unwrap_or(0)
                        }
                    } else {
                        u1[start_idx..]
                            .windows(u2.len())
                            .position(|window| window == u2.as_slice())
                            .map(|pos| (start_idx + pos + 1) as i32)
                            .unwrap_or(0)
                    }
                };
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_I4;
                out_var.data[0..4].copy_from_slice(&(idx as u32).to_le_bytes());
                if dst_ptr != 0 {
                    self.mem.write_variant(dst_ptr, &out_var);
                }
                self.regs.eax = dst_ptr;
            }
            130 => {
                // __vbaFreeStr(p_bstr in ecx)
                let p = self.regs.ecx;
                if p != 0 {
                    self.mem.write_u32(p, 0);
                }
                self.regs.eax = 0;
            }
            244 => {
                // __vbaFreeStrList(count: i32, ...) - __cdecl, caller cleans stack
                let count = self.mem.read_u32(self.regs.esp) as usize;
                for i in 0..count {
                    let p = self.mem.read_u32(self.regs.esp.wrapping_add(4 + (i as u32 * 4)));
                    if p != 0 {
                        self.mem.write_u32(p, 0);
                    }
                }
                self.regs.eax = 0;
            }
            394 => {
                // __vbaSetSystemError(err: i32 in EAX or no-op)
                self.regs.eax = 0;
            }
            406 => {
                // __vbaStrCmp(s1: Bstr, s2: Bstr) -> i32: push s1; push s2; -> [esp] = s2, [esp+4] = s1
                let s2 = self.mem.read_u32(self.regs.esp);
                let s1 = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let c1 = self.mem.read_bstr(s1);
                let c2 = self.mem.read_bstr(s2);
                let res = match c1.cmp(&c2) {
                    core::cmp::Ordering::Less => -1i32 as u32,
                    core::cmp::Ordering::Equal => 0,
                    core::cmp::Ordering::Greater => 1,
                };
                self.regs.eax = res;
            }
            519 => {
                // rtcTrimBstr(s: Bstr) -> Bstr
                let s = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let chars = self.mem.read_bstr(s);
                let mut start = 0;
                while start < chars.len() && chars[start] <= 0x0020 {
                    start += 1;
                }
                let mut end = chars.len();
                while end > start && chars[end - 1] <= 0x0020 {
                    end -= 1;
                }
                let trimmed = &chars[start..end];
                let res = self.mem.alloc_bstr(trimmed);
                self.regs.eax = res;
            }
            521 => {
                // rtcLeftTrimBstr(s: Bstr) -> Bstr
                let s = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let chars = self.mem.read_bstr(s);
                let mut start = 0;
                while start < chars.len() && chars[start] <= 0x0020 {
                    start += 1;
                }
                let trimmed = &chars[start..];
                let res = self.mem.alloc_bstr(trimmed);
                self.regs.eax = res;
            }
            523 => {
                // rtcRightTrimBstr(s: Bstr) -> Bstr
                let s = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let chars = self.mem.read_bstr(s);
                let mut end = chars.len();
                while end > 0 && chars[end - 1] <= 0x0020 {
                    end -= 1;
                }
                let trimmed = &chars[..end];
                let res = self.mem.alloc_bstr(trimmed);
                self.regs.eax = res;
            }
            517 => {
                // rtcLowerCaseBstr(s: Bstr) -> Bstr
                let s = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let chars: Vec<u16> = self.mem.read_bstr(s).into_iter().map(|c| {
                    if (c >= b'A' as u16) && (c <= b'Z' as u16) { c + 32 } else { c }
                }).collect();
                let res = self.mem.alloc_bstr(&chars);
                self.regs.eax = res;
            }
            527 => {
                // rtcUpperCaseBstr(s: Bstr) -> Bstr
                let s = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let chars: Vec<u16> = self.mem.read_bstr(s).into_iter().map(|c| {
                    if (c >= b'a' as u16) && (c <= b'z' as u16) { c - 32 } else { c }
                }).collect();
                let res = self.mem.alloc_bstr(&chars);
                self.regs.eax = res;
            }
            525 => {
                // rtcSpaceBstr(n: i32) -> Bstr
                let n = self.mem.read_u32(self.regs.esp) as usize;
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let chars = vec![0x0020u16; n];
                let res = self.mem.alloc_bstr(&chars);
                self.regs.eax = res;
            }
            333 => {
                // __vbaLsetFixstr(char_len: usize, dst: *mut u16, src: Bstr)
                let mut char_len = self.mem.read_u32(self.regs.esp) as usize;
                let dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let src = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                if char_len == 0 && dst != 0 {
                    char_len = (self.mem.read_u32(dst.wrapping_sub(4)) / 2) as usize;
                }
                let src_chars = self.mem.read_bstr(src);
                for i in 0..char_len {
                    let ch = if i < src_chars.len() { src_chars[i] } else { 0x0020 };
                    self.mem.write_u16(dst.wrapping_add(i as u32 * 2), ch);
                }
                self.regs.eax = dst;
            }
            392 => {
                // __vbaRsetFixstr(char_len: usize, dst: *mut u16, src: Bstr)
                let mut char_len = self.mem.read_u32(self.regs.esp) as usize;
                let dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let src = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                if char_len == 0 && dst != 0 {
                    char_len = (self.mem.read_u32(dst.wrapping_sub(4)) / 2) as usize;
                }
                let src_chars = self.mem.read_bstr(src);
                let pad = if char_len > src_chars.len() { char_len - src_chars.len() } else { 0 };
                for i in 0..char_len {
                    let ch = if i < pad { 0x0020 } else { src_chars[i - pad] };
                    self.mem.write_u16(dst.wrapping_add(i as u32 * 2), ch);
                }
                self.regs.eax = dst;
            }
            416 => {
                // __vbaStrFixstr(char_len: usize, src_buf: *const u16) -> Bstr
                let char_len = self.mem.read_u32(self.regs.esp) as usize;
                let src_buf = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let mut chars = Vec::with_capacity(char_len);
                for i in 0..char_len {
                    chars.push(self.mem.read_u16(src_buf.wrapping_add(i as u32 * 2)));
                }
                let res = self.mem.alloc_bstr(&chars);
                self.regs.eax = res;
            }
            349 | 350 => {
                // __vbaObjSet (349), __vbaObjSetAddref (350) (dst: *mut *mut c_void, src: *mut c_void)
                let dst = self.mem.read_u32(self.regs.esp);
                let src = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if dst != 0 {
                    self.mem.write_u32(dst, src);
                }
                self.regs.eax = src;
            }
            178 => {
                // __vbaCastObj(p_obj: *mut c_void, p_iid: *const Iid) (pops 8 bytes)
                let p_obj = self.mem.read_u32(self.regs.esp);
                let _p_iid = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.regs.eax = p_obj;
            }
            179 => {
                // __vbaCastObjVar(p_var: *const Variant, p_iid: *const Iid) (pops 8 bytes)
                let p_var = self.mem.read_u32(self.regs.esp);
                let _p_iid = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if p_var != 0 {
                    let var = self.mem.read_variant_deref(p_var);
                    let obj = if (var.vt & !crate::types::variant::VT_BYREF) == crate::types::VT_DISPATCH
                        || (var.vt & !crate::types::variant::VT_BYREF) == crate::types::VT_UNKNOWN
                    {
                        u32::from_le_bytes(var.data[0..4].try_into().unwrap_or([0; 4]))
                    } else {
                        0
                    };
                    self.regs.eax = obj;
                } else {
                    self.regs.eax = 0;
                }
            }
            183 => {
                // __vbaCopyBytes(count: usize, dst: *mut u8, src: *const u8)
                let count = self.mem.read_u32(self.regs.esp) as usize;
                let dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let src = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                for i in 0..count {
                    let b = self.mem.read_u8(src.wrapping_add(i as u32));
                    self.mem.write_u8(dst.wrapping_add(i as u32), b);
                }
                self.regs.eax = dst;
            }
            381 | 190 => {
                // __vbaRecAssign, CopyRecord
                let info = self.mem.read_u32(self.regs.esp);
                let dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let src = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);

                let cb_record = if info != 0 {
                    self.mem.read_u16(info.wrapping_add(2)) as usize
                } else {
                    64
                };
                let count = if cb_record > 0 && cb_record <= 1024 { cb_record } else { 64 };
                for i in 0..count {
                    let b = self.mem.read_u8(src.wrapping_add(i as u32));
                    self.mem.write_u8(dst.wrapping_add(i as u32), b);
                }

                if info != 0 {
                    let c_fields = self.mem.read_u16(info.wrapping_add(6)) as usize;
                    let mut pos = 12u32;
                    for _ in 0..c_fields {
                        let uni_offset = self.mem.read_u16(info.wrapping_add(pos)) as u32;
                        let ftype = self.mem.read_u16(info.wrapping_add(pos + 2));
                        let base_type = ftype & 0x0FFF;
                        let has_ext = (ftype & 0x2000) != 0;
                        if base_type == 0x0001 {
                            let src_bstr = self.mem.read_u32(src.wrapping_add(uni_offset));
                            let cloned = if src_bstr != 0 {
                                let chars = self.mem.read_bstr(src_bstr);
                                self.mem.alloc_bstr(&chars)
                            } else {
                                0
                            };
                            self.mem.write_u32(dst.wrapping_add(uni_offset), cloned);
                            pos += if has_ext { 8 } else { 4 };
                        } else if base_type == 0x0105 {
                            let src_sa = self.mem.read_u32(src.wrapping_add(uni_offset));
                            let cloned_sa = if src_sa != 0 {
                                let c_dims = self.mem.read_u16(src_sa);
                                let f_features = self.mem.read_u16(src_sa.wrapping_add(2));
                                let cb_elements = self.mem.read_u32(src_sa.wrapping_add(4));
                                let src_data = self.mem.read_u32(src_sa.wrapping_add(12));
                                let c_elements = self.mem.read_u32(src_sa.wrapping_add(16));
                                let l_lbound = self.mem.read_u32(src_sa.wrapping_add(20));

                                let total_bytes = (cb_elements * c_elements) as usize;
                                let dst_data = self.mem.alloc_bytes(total_bytes);
                                for i in 0..total_bytes {
                                    let b = self.mem.read_u8(src_data.wrapping_add(i as u32));
                                    self.mem.write_u8(dst_data.wrapping_add(i as u32), b);
                                }
                                let dst_sa = self.mem.alloc_bytes(24);
                                self.mem.write_u16(dst_sa, c_dims);
                                self.mem.write_u16(dst_sa.wrapping_add(2), f_features);
                                self.mem.write_u32(dst_sa.wrapping_add(4), cb_elements);
                                self.mem.write_u32(dst_sa.wrapping_add(8), 0);
                                self.mem.write_u32(dst_sa.wrapping_add(12), dst_data);
                                self.mem.write_u32(dst_sa.wrapping_add(16), c_elements);
                                self.mem.write_u32(dst_sa.wrapping_add(20), l_lbound);
                                dst_sa
                            } else {
                                0
                            };
                            self.mem.write_u32(dst.wrapping_add(uni_offset), cloned_sa);
                            pos += 10;
                        } else if base_type == 0x000A {
                            pos += 10;
                        } else if base_type == 0x000B {
                            pos += 8;
                        } else {
                            pos += if has_ext { 8 } else { 4 };
                        }
                    }
                }
                self.regs.eax = dst;
            }
            170 => {
                // __vbaAryRecCopy(info: u32, pp_dst: u32, pp_src: u32)
                let info = self.mem.read_u32(self.regs.esp);
                let pp_dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let pp_src = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);

                if pp_dst != 0 {
                    let src_sa = if pp_src != 0 { self.mem.read_u32(pp_src) } else { 0 };
                    if src_sa == 0 {
                        self.mem.write_u32(pp_dst, 0);
                    } else {
                        let c_dims = self.mem.read_u16(src_sa);
                        let f_features = self.mem.read_u16(src_sa.wrapping_add(2));
                        let cb_elements = self.mem.read_u32(src_sa.wrapping_add(4));
                        let src_data = self.mem.read_u32(src_sa.wrapping_add(12));
                        let c_elements = self.mem.read_u32(src_sa.wrapping_add(16));
                        let l_lbound = self.mem.read_u32(src_sa.wrapping_add(20));

                        let total_bytes = (cb_elements * c_elements) as usize;
                        let dst_data = self.mem.alloc_bytes(total_bytes);

                        for i in 0..total_bytes {
                            let b = self.mem.read_u8(src_data.wrapping_add(i as u32));
                            self.mem.write_u8(dst_data.wrapping_add(i as u32), b);
                        }

                        if info != 0 {
                            let c_fields = self.mem.read_u16(info.wrapping_add(6)) as usize;
                            for elem_idx in 0..c_elements {
                                let elem_src = src_data.wrapping_add(elem_idx * cb_elements);
                                let elem_dst = dst_data.wrapping_add(elem_idx * cb_elements);
                                let mut pos = 12u32;
                                for _ in 0..c_fields {
                                    let uni_offset = self.mem.read_u16(info.wrapping_add(pos)) as u32;
                                    let ftype = self.mem.read_u16(info.wrapping_add(pos + 2));
                                    let base_type = ftype & 0x0FFF;
                                    let has_ext = (ftype & 0x2000) != 0;
                                    if base_type == 0x0001 {
                                        let src_bstr = self.mem.read_u32(elem_src.wrapping_add(uni_offset));
                                        let cloned = if src_bstr != 0 {
                                            let chars = self.mem.read_bstr(src_bstr);
                                            self.mem.alloc_bstr(&chars)
                                        } else {
                                            0
                                        };
                                        self.mem.write_u32(elem_dst.wrapping_add(uni_offset), cloned);
                                        pos += if has_ext { 8 } else { 4 };
                                    } else if base_type == 0x0105 {
                                        let field_sa = self.mem.read_u32(elem_src.wrapping_add(uni_offset));
                                        let cloned_sa = if field_sa != 0 {
                                            let f_dims = self.mem.read_u16(field_sa);
                                            let f_feat = self.mem.read_u16(field_sa.wrapping_add(2));
                                            let f_cb = self.mem.read_u32(field_sa.wrapping_add(4));
                                            let f_src_data = self.mem.read_u32(field_sa.wrapping_add(12));
                                            let f_count = self.mem.read_u32(field_sa.wrapping_add(16));
                                            let f_lbound = self.mem.read_u32(field_sa.wrapping_add(20));

                                            let f_bytes = (f_cb * f_count) as usize;
                                            let f_dst_data = self.mem.alloc_bytes(f_bytes);
                                            for i in 0..f_bytes {
                                                let b = self.mem.read_u8(f_src_data.wrapping_add(i as u32));
                                                self.mem.write_u8(f_dst_data.wrapping_add(i as u32), b);
                                            }
                                            let f_dst_sa = self.mem.alloc_bytes(24);
                                            self.mem.write_u16(f_dst_sa, f_dims);
                                            self.mem.write_u16(f_dst_sa.wrapping_add(2), f_feat);
                                            self.mem.write_u32(f_dst_sa.wrapping_add(4), f_cb);
                                            self.mem.write_u32(f_dst_sa.wrapping_add(8), 0);
                                            self.mem.write_u32(f_dst_sa.wrapping_add(12), f_dst_data);
                                            self.mem.write_u32(f_dst_sa.wrapping_add(16), f_count);
                                            self.mem.write_u32(f_dst_sa.wrapping_add(20), f_lbound);
                                            f_dst_sa
                                        } else {
                                            0
                                        };
                                        self.mem.write_u32(elem_dst.wrapping_add(uni_offset), cloned_sa);
                                        pos += 10;
                                    } else if base_type == 0x000A {
                                        pos += 10;
                                    } else if base_type == 0x000B {
                                        pos += 8;
                                    } else {
                                        pos += if has_ext { 8 } else { 4 };
                                    }
                                }
                            }
                        }

                        let dst_sa = self.mem.alloc_bytes(24);
                        self.mem.write_u16(dst_sa, c_dims);
                        self.mem.write_u16(dst_sa.wrapping_add(2), f_features);
                        self.mem.write_u32(dst_sa.wrapping_add(4), cb_elements);
                        self.mem.write_u32(dst_sa.wrapping_add(8), 0);
                        self.mem.write_u32(dst_sa.wrapping_add(12), dst_data);
                        self.mem.write_u32(dst_sa.wrapping_add(16), c_elements);
                        self.mem.write_u32(dst_sa.wrapping_add(20), l_lbound);

                        self.mem.write_u32(pp_dst, dst_sa);
                    }
                }
                self.regs.eax = 0;
            }
            385 => {
                // __vbaRedim (cdecl, caller pops 28 bytes)
                let f_features = self.mem.read_u32(self.regs.esp) as u16;
                let elem_size = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let pp_array = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let ubound = self.mem.read_u32(self.regs.esp.wrapping_add(20)) as i32;
                let lbound = self.mem.read_u32(self.regs.esp.wrapping_add(24)) as i32;

                let count = if ubound >= lbound { (ubound - lbound + 1) as u32 } else { 0 };
                let total_bytes = (elem_size * count) as usize;
                let pv_data = self.mem.alloc_bytes(total_bytes);

                let sa_ptr = self.mem.alloc_bytes(24);
                self.mem.write_u16(sa_ptr, 1);
                self.mem.write_u16(sa_ptr.wrapping_add(2), f_features);
                self.mem.write_u32(sa_ptr.wrapping_add(4), elem_size);
                self.mem.write_u32(sa_ptr.wrapping_add(8), 0);
                self.mem.write_u32(sa_ptr.wrapping_add(12), pv_data);
                self.mem.write_u32(sa_ptr.wrapping_add(16), count);
                self.mem.write_u32(sa_ptr.wrapping_add(20), lbound as u32);

                if pp_array != 0 {
                    self.mem.write_u32(pp_array, sa_ptr);
                }
                self.regs.eax = 0;
            }
            386 => {
                // __vbaRedimPreserve (cdecl, caller pops 28 bytes)
                let f_features = self.mem.read_u32(self.regs.esp) as u16;
                let elem_size = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let pp_array = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let ubound = self.mem.read_u32(self.regs.esp.wrapping_add(20)) as i32;
                let lbound = self.mem.read_u32(self.regs.esp.wrapping_add(24)) as i32;

                let new_count = if ubound >= lbound { (ubound - lbound + 1) as u32 } else { 0 };
                let existing_sa = if pp_array != 0 { self.mem.read_u32(pp_array) } else { 0 };

                if existing_sa == 0 {
                    let total_bytes = (elem_size * new_count) as usize;
                    let pv_data = self.mem.alloc_bytes(total_bytes);

                    let sa_ptr = self.mem.alloc_bytes(24);
                    self.mem.write_u16(sa_ptr, 1);
                    self.mem.write_u16(sa_ptr.wrapping_add(2), f_features);
                    self.mem.write_u32(sa_ptr.wrapping_add(4), elem_size);
                    self.mem.write_u32(sa_ptr.wrapping_add(8), 0);
                    self.mem.write_u32(sa_ptr.wrapping_add(12), pv_data);
                    self.mem.write_u32(sa_ptr.wrapping_add(16), new_count);
                    self.mem.write_u32(sa_ptr.wrapping_add(20), lbound as u32);

                    if pp_array != 0 {
                        self.mem.write_u32(pp_array, sa_ptr);
                    }
                } else {
                    let old_count = self.mem.read_u32(existing_sa.wrapping_add(16));
                    let old_data = self.mem.read_u32(existing_sa.wrapping_add(12));
                    let new_data = self.mem.alloc_bytes((elem_size * new_count) as usize);

                    let copy_bytes = (old_count.min(new_count) * elem_size) as usize;
                    for i in 0..copy_bytes {
                        let b = self.mem.read_u8(old_data.wrapping_add(i as u32));
                        self.mem.write_u8(new_data.wrapping_add(i as u32), b);
                    }

                    self.mem.write_u32(existing_sa.wrapping_add(12), new_data);
                    self.mem.write_u32(existing_sa.wrapping_add(16), new_count);
                    self.mem.write_u32(existing_sa.wrapping_add(20), lbound as u32);
                }
                self.regs.eax = 0;
            }
            445 => {
                // __vbaUbound (stdcall: [esp] = dim, [esp+4] = p_array) -> i32
                let dim = self.mem.read_u32(self.regs.esp) as i32;
                let p_array = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if p_array == 0 {
                    self.regs.eax = 0xFFFFFFFF;
                } else {
                    let c_dims = self.mem.read_u16(p_array) as i32;
                    if dim < 1 || dim > c_dims {
                        self.regs.eax = 0xFFFFFFFF;
                    } else {
                        let c_elements = self.mem.read_u32(p_array.wrapping_add(16 + (dim as u32 - 1) * 8));
                        let l_lbound = self.mem.read_u32(p_array.wrapping_add(20 + (dim as u32 - 1) * 8)) as i32;
                        self.regs.eax = (l_lbound + c_elements as i32 - 1) as u32;
                    }
                }
            }
            325 => {
                // __vbaLbound (stdcall: [esp] = dim, [esp+4] = p_array) -> i32
                let dim = self.mem.read_u32(self.regs.esp) as i32;
                let p_array = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if p_array == 0 {
                    self.regs.eax = 0;
                } else {
                    let c_dims = self.mem.read_u16(p_array) as i32;
                    if dim < 1 || dim > c_dims {
                        self.regs.eax = 0;
                    } else {
                        let l_lbound = self.mem.read_u32(p_array.wrapping_add(20 + (dim as u32 - 1) * 8));
                        self.regs.eax = l_lbound;
                    }
                }
            }
            213 => {
                // __vbaErase(rec_info: u32, pp_array: *mut *mut SafeArray) (stdcall, pops 8 bytes)
                let _rec_info = self.mem.read_u32(self.regs.esp);
                let pp_array = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if pp_array != 0 {
                    self.mem.write_u32(pp_array, 0);
                }
                self.regs.eax = 0;
            }
            246 => {
                // __vbaGenerateBoundsError
                return self.handle_error(9);
            }
            235 => {
                // __vbaFileOpen(mode: i32, record_len: i32, file_number: i32, filename: Bstr) (stdcall)
                let mode = self.mem.read_u32(self.regs.esp) as i32;
                let rec_len = self.mem.read_u32(self.regs.esp.wrapping_add(4)) as i32;
                let file_num = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as i32;
                let fn_bstr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                self.regs.esp = self.regs.esp.wrapping_add(16);

                let chars = self.mem.read_bstr(fn_bstr);
                let (res, _) = unsafe {
                    let path_bstr = crate::types::bstr::SysAllocStringLen(chars.as_ptr(), chars.len() as u32);
                    let r = crate::vba::__vbaFileOpen(mode, rec_len, file_num, path_bstr);
                    crate::types::bstr::SysFreeString(path_bstr);
                    (r, ())
                };
                self.regs.eax = res as u32;
            }
            229 => {
                // __vbaFileClose(file_number: i32) (stdcall)
                let file_num = self.mem.read_u32(self.regs.esp) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = unsafe { crate::vba::__vbaFileClose(file_num) } as u32;
            }
            236 => {
                // __vbaFileSeek(file_number: i32, pos: i32) (stdcall)
                let file_num = self.mem.read_u32(self.regs.esp) as i32;
                let pos = self.mem.read_u32(self.regs.esp.wrapping_add(4)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.regs.eax = unsafe { crate::vba::__vbaFileSeek(file_num, pos) } as u32;
            }
            360 => {
                // __vbaPutFxStr4(char_len: usize, str_ptr: *const u16, pos: i32, file_num: i32) (stdcall)
                let char_len = self.mem.read_u32(self.regs.esp) as usize;
                let str_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let pos = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as i32;
                let file_num = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(16);

                let mut chars = vec![0u16; char_len];
                for i in 0..char_len {
                    chars[i] = self.mem.read_u16(str_ptr.wrapping_add(i as u32 * 2));
                }
                self.regs.eax = unsafe { crate::vba::__vbaPutFxStr4(char_len as i32, chars.as_ptr(), pos, file_num) } as u32;
            }
            250 => {
                // __vbaGetFxStr4(char_len: usize, str_ptr: *mut u16, pos: i32, file_num: i32) (stdcall)
                let char_len = self.mem.read_u32(self.regs.esp) as usize;
                let str_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let pos = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as i32;
                let file_num = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(16);

                let mut chars = vec![0u16; char_len];
                let res = unsafe { crate::vba::__vbaGetFxStr4(char_len as i32, chars.as_mut_ptr(), pos, file_num) };
                for i in 0..char_len {
                    self.mem.write_u16(str_ptr.wrapping_add(i as u32 * 2), chars[i]);
                }
                self.regs.eax = res as u32;
            }
            361 => {
                // __vbaPutOwner3(owner_desc: *const c_void, p_rec: *const c_void, file_num: i32) (stdcall)
                let owner_desc = self.mem.read_u32(self.regs.esp);
                let p_rec = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let file_num = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(12);

                let cb_record = if owner_desc != 0 {
                    let w2 = self.mem.read_u16(owner_desc.wrapping_add(2)) as usize;
                    if w2 != 0 {
                        w2
                    } else {
                        self.mem.read_u16(owner_desc) as usize
                    }
                } else {
                    64
                };
                let mut buf = vec![0u8; cb_record];
                for i in 0..cb_record {
                    buf[i] = self.mem.read_u8(p_rec.wrapping_add(i as u32));
                }
                self.regs.eax = unsafe { crate::vba::__vbaPut4(cb_record as i32, buf.as_ptr() as *const _, -1, file_num) } as u32;
            }
            251 => {
                // __vbaGetOwner3(owner_desc: *const c_void, p_rec: *mut c_void, file_num: i32) (stdcall)
                let owner_desc = self.mem.read_u32(self.regs.esp);
                let p_rec = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let file_num = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(12);

                let cb_record = if owner_desc != 0 {
                    let w2 = self.mem.read_u16(owner_desc.wrapping_add(2)) as usize;
                    if w2 != 0 {
                        w2
                    } else {
                        self.mem.read_u16(owner_desc) as usize
                    }
                } else {
                    64
                };
                let mut buf = vec![0u8; cb_record];
                let res = unsafe { crate::vba::__vbaGet4(cb_record as i32, buf.as_mut_ptr() as *mut _, -1, file_num) };
                for i in 0..cb_record {
                    self.mem.write_u8(p_rec.wrapping_add(i as u32), buf[i]);
                }
                self.regs.eax = res as u32;
            }
            648 => {
                // rtcFreeFile(range_number: *const Variant) -> i16 (stdcall)
                let _range = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = unsafe { crate::rtc::rtcFreeFile(core::ptr::null()) } as u16 as u32;
            }
            645 => {
                // rtcDir(pathname: *const Variant, _attr: i32) -> Bstr (stdcall)
                let pathname = self.mem.read_u32(self.regs.esp);
                let _attr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);

                // If pathname is a string, check if it exists
                let bstr_ptr = if pathname != 0 { self.mem.read_u32(pathname.wrapping_add(8)) } else { 0 };
                let exists = if bstr_ptr != 0 {
                    let chars = self.mem.read_bstr(bstr_ptr);
                    let path = String::from_utf16_lossy(&chars);
                    std::path::Path::new(&path).exists()
                } else {
                    false
                };
                if exists && bstr_ptr != 0 {
                    let chars = self.mem.read_bstr(bstr_ptr);
                    self.regs.eax = self.mem.alloc_bstr(&chars);
                } else {
                    self.regs.eax = 0;
                }
            }
            529 => {
                // rtcKillFiles(path: Bstr) (stdcall)
                let path_bstr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if path_bstr != 0 {
                    let chars = self.mem.read_bstr(path_bstr);
                    let path = String::from_utf16_lossy(&chars);
                    let _ = std::fs::remove_file(path);
                }
                self.regs.eax = 0;
            }
            728 => {
                // _allmul(A: i64, B: i64) -> EDX:EAX (pops 16 bytes)
                let a_lo = self.mem.read_u32(self.regs.esp) as u64;
                let a_hi = self.mem.read_u32(self.regs.esp.wrapping_add(4)) as u64;
                let b_lo = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as u64;
                let b_hi = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as u64;
                self.regs.esp = self.regs.esp.wrapping_add(16);
                let a = (a_hi << 32) | a_lo;
                let b = (b_hi << 32) | b_lo;
                let res = a.wrapping_mul(b);
                self.regs.eax = res as u32;
                self.regs.edx = (res >> 32) as u32;
            }
            0x8006 => {
                // GetStartupInfoA(lpStartupInfo: *mut STARTUPINFOA) (stdcall)
                let lp_si = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if lp_si != 0 {
                    let cb = self.mem.read_u32(lp_si);
                    if cb == 0 {
                        self.mem.write_u32(lp_si, 68);
                    }
                }
                self.regs.eax = 0;
            }
            382 | 383 => {
                // __vbaRecDestruct, __vbaRecDestructAnsi
                let _info = self.mem.read_u32(self.regs.esp);
                let rec = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.regs.eax = rec;
            }
            380 | 384 => {
                // __vbaRecAnsiToUni, __vbaRecUniToAnsi
                let info = self.mem.read_u32(self.regs.esp);
                let dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let src = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);

                let cb_record = if info != 0 {
                    self.mem.read_u16(info.wrapping_add(2)) as usize
                } else {
                    64
                };
                let count = if cb_record > 0 && cb_record <= 1024 { cb_record } else { 64 };
                for i in 0..count {
                    let b = self.mem.read_u8(src.wrapping_add(i as u32));
                    self.mem.write_u8(dst.wrapping_add(i as u32), b);
                }
                self.regs.eax = dst;
            }
            127 => {
                // __vbaFpR8(val: f64 in ST0) -> ST0 (pass-through)
            }
            109 => {
                // __vbaBoolErrVar(p_var: *const Variant) -> i16 (pops 4 bytes)
                let p_var = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if p_var != 0 {
                    let var = self.mem.read_variant_deref(p_var);
                    let vt = var.vt & !crate::types::variant::VT_BYREF;
                    if vt == crate::types::variant::VT_ERROR {
                        self.regs.eax = 0;
                        self.handle_error(13)?;
                    } else {
                        let is_true = match vt {
                            crate::types::VT_EMPTY | crate::types::VT_NULL => false,
                            crate::types::VT_BOOL | crate::types::VT_I2 => i16::from_le_bytes([var.data[0], var.data[1]]) != 0,
                            crate::types::VT_I4 => i32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) != 0,
                            crate::types::VT_UI1 => var.data[0] != 0,
                            crate::types::VT_R4 => f32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) != 0.0,
                            crate::types::VT_R8 | crate::types::VT_DATE => f64::from_le_bytes(var.data[0..8].try_into().unwrap()) != 0.0,
                            crate::types::VT_CY => i64::from_le_bytes(var.data[0..8].try_into().unwrap()) != 0,
                            _ => var.as_bool().unwrap_or(false),
                        };
                        self.regs.eax = if is_true { 0xFFFF } else { 0 };
                    }
                } else {
                    self.regs.eax = 0;
                }
            }
            118 | 119 | 126 => {
                // __vbaFpCDblR4 / __vbaFpCDblR8 / __vbaFpR4 (pass-through in ST0)
            }
            120 | 121 => {
                // __vbaFpCSngR4 / __vbaFpCSngR8 (rounds ST0 to single)
                let v = self.fpu.pop();
                self.fpu.push((v as f32) as f64);
            }
            123 => {
                // __vbaFpCy(val in ST0) -> Currency (i64) in EDX:EAX (pops ST0)
                let val = self.fpu.pop();
                let cy = (val * 10000.0).round() as i64;
                self.regs.eax = cy as u32;
                self.regs.edx = (cy >> 32) as u32;
            }
            124 => {
                // __vbaFpI2(val in ST0) -> i16 in AX (pops ST0)
                let val = self.fpu.pop();
                let r = crate::math::round_to_even(val);
                if r < (i16::MIN as f64) || r > (i16::MAX as f64) {
                    return self.handle_error(6);
                }
                self.regs.eax = (self.regs.eax & 0xFFFF0000) | ((r as i16 as u16) as u32);
            }
            125 => {
                // __vbaFpI4(val in ST0) -> i32 in EAX (pops ST0)
                let val = self.fpu.pop();
                let r = crate::math::round_to_even(val);
                if r < (i32::MIN as f64) || r > (i32::MAX as f64) {
                    return self.handle_error(6);
                }
                self.regs.eax = (r as i32) as u32;
            }
            128 => {
                // __vbaFpUI1(val in ST0) -> u8 in AL (pops ST0)
                let v = self.fpu.pop();
                let rounded = crate::math::round_to_even(v);
                self.regs.eax = (rounded as u8) as u32;
            }
            133 => {
                // __vbaI2Abs(val in cx) -> ax
                let val = self.regs.ecx as i16;
                self.regs.eax = val.unsigned_abs() as u32;
            }
            135 => {
                // __vbaI2Sgn(val in cx) -> ax
                let val = self.regs.ecx as i16;
                let sgn: i16 = if val > 0 { 1 } else if val < 0 { -1 } else { 0 };
                self.regs.eax = (sgn as u16) as u32;
            }
            136 => {
                // __vbaI4Abs(val in ecx) -> eax
                let val = self.regs.ecx as i32;
                self.regs.eax = val.unsigned_abs();
            }
            137 => {
                // __vbaI4Sgn(val in ecx) -> ax
                let val = self.regs.ecx as i32;
                let sgn: i16 = if val > 0 { 1 } else if val < 0 { -1 } else { 0 };
                self.regs.eax = (sgn as u16) as u32;
            }
            195 => {
                // TipGetAddressOfPredeclaredInstance(p_desc, p_inst) (pops 8 bytes)
                let p_desc = self.mem.read_u32(self.regs.esp);
                let p_inst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if p_inst != 0 {
                    let obj = if p_desc != 0 { self.mem.read_u32(p_desc) } else { 0 };
                    let final_obj = if obj != 0 {
                        obj
                    } else {
                        self.register_host_com_object(crate::ide::get_ide_com_object_ptr() as usize)
                    };
                    self.mem.write_u32(p_inst, final_obj);
                }
                self.regs.eax = 0;
            }
            220 => {
                // TipSetOption(opt_id, opt_val) (pops 8 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.regs.eax = 0;
            }
            227 => {
                // TipUnloadProject(hproj) (pops 4 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = 0;
            }
            230 => {
                // TipCreateInstanceProject2(hproj, clsid, out_inst) (pops 12 bytes)
                let _hproj = self.mem.read_u32(self.regs.esp);
                let _clsid = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let out_inst = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                if out_inst != 0 {
                    let obj = self.register_host_com_object(crate::ide::get_ide_com_object_ptr() as usize);
                    self.mem.write_u32(out_inst, obj);
                }
                self.regs.eax = 0;
            }
            231 => {
                // EbResetProject() (pops 0 bytes)
                self.last_error_number = 0;
                self.last_error_source.clear();
                self.last_error_description.clear();
                self.regs.eax = 0;
            }
            232 => {
                // EbGetHandleOfExecutingProject(out_hproject) (pops 4 bytes)
                let out_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if out_ptr != 0 {
                    let base = unsafe { crate::pe::shims::GLOBAL_MODULE_HANDLE };
                    self.mem.write_u32(out_ptr, if base != 0 { base as u32 } else { 0x00400000 });
                }
                self.regs.eax = 0;
            }
            265 => {
                // EbResetProjectNormal() (pops 0 bytes)
                self.last_error_number = 0;
                self.last_error_source.clear();
                self.last_error_description.clear();
                self.regs.eax = 0;
            }
            266 => {
                // TipUnloadInstance(p_obj) (pops 4 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = 0;
            }
            268 => {
                // EbLibraryLoad(p_lib_name) (pops 4 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = 1;
            }
            269 => {
                // EbLibraryUnload(h_lib) (pops 4 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = 0;
            }
            271 => {
                // EbLoadRunTime() (pops 0 bytes)
                self.regs.eax = 0;
            }
            284 => {
                // EbGetErrorInfo(err_num, pbstr_desc) (pops 8 bytes)
                let err_num_ptr = self.mem.read_u32(self.regs.esp);
                let _pbstr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if err_num_ptr != 0 {
                    self.mem.write_u32(err_num_ptr, self.last_error_number);
                }
                self.regs.eax = 0;
            }
            298 => {
                // EbIsProjectOnStack(hproject) (pops 4 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = 1;
            }
            299 => {
                // TipCreateInstanceEx(hproj, clsid, p_obj) (pops 12 bytes)
                let _hproj = self.mem.read_u32(self.regs.esp);
                let _clsid = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let p_obj = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                if p_obj != 0 {
                    let obj = self.register_host_com_object(crate::ide::get_ide_com_object_ptr() as usize);
                    self.mem.write_u32(p_obj, obj);
                }
                self.regs.eax = 0;
            }
            999 => {
                // TipInvokeMethod2(p_obj, method_id, extra) (pops 12 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(12);
                self.regs.eax = 0;
            }
            1016 => {
                // TipInvokeMethod(p_obj, method_id) (pops 8 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.regs.eax = 0;
            }
            1025 => {
                // EbGetObjConnectionCounts(p_obj, out_count) (pops 8 bytes)
                let _p_obj = self.mem.read_u32(self.regs.esp);
                let out_count = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if out_count != 0 {
                    self.mem.write_u32(out_count, 0);
                }
                self.regs.eax = 0;
            }
            2000 => {
                // CreateIExprSrvObj(p_unk_outer, riid, p_expr_srv) (pops 12 bytes)
                let _outer = self.mem.read_u32(self.regs.esp);
                let _riid = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let p_expr_srv = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                if p_expr_srv != 0 {
                    let obj = self.register_host_com_object(crate::ide::get_ide_com_object_ptr() as usize);
                    self.mem.write_u32(p_expr_srv, obj);
                }
                self.regs.eax = 0;
            }
            2010 => {
                // EbGetVBAObject(p_vba) (pops 4 bytes)
                let p_vba = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if p_vba != 0 {
                    let obj = self.register_host_com_object(crate::ide::get_ide_com_object_ptr() as usize);
                    self.mem.write_u32(p_vba, obj);
                }
                self.regs.eax = 0;
            }
            175 | 176 => {
                // __vbaBoolVar (175) / __vbaBoolVarNull (176)
                // (p_var: *const Variant) -> i16 in AX (pops 4 bytes)
                let p_var = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if p_var != 0 {
                    let var = self.mem.read_variant_deref(p_var);
                    let vt = (var.vt & 0x0FFF) & !crate::types::variant::VT_BYREF;
                    if vt == crate::types::VT_NULL {
                        if ordinal == 175 {
                            self.handle_error(94)?;
                        }
                        self.regs.eax = 0;
                    } else if vt == crate::types::VT_EMPTY {
                        self.regs.eax = 0;
                    } else {
                        let is_true = match vt {
                            crate::types::VT_BOOL | crate::types::VT_I2 => i16::from_le_bytes([var.data[0], var.data[1]]) != 0,
                            crate::types::VT_I4 => i32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) != 0,
                            crate::types::VT_UI1 => var.data[0] != 0,
                            crate::types::VT_R4 => f32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) != 0.0,
                            crate::types::VT_R8 | crate::types::VT_DATE => f64::from_le_bytes(var.data[0..8].try_into().unwrap()) != 0.0,
                            crate::types::VT_CY => i64::from_le_bytes(var.data[0..8].try_into().unwrap()) != 0,
                            _ => var.as_bool().unwrap_or(false),
                        };
                        self.regs.eax = if is_true { 0xFFFF } else { 0 };
                    }
                } else {
                    self.regs.eax = 0;
                }
            }
            185 => {
                // __vbaCyAbs(val: i64 on stack) -> i64 in EDX:EAX (pops 8 bytes)
                let low = self.mem.read_u32(self.regs.esp);
                let high = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let val = (((high as u64) << 32) | (low as u64)) as i64;
                let abs_val = val.abs();
                self.regs.eax = abs_val as u32;
                self.regs.edx = (abs_val as u64 >> 32) as u32;
            }
            189 => {
                // __vbaCyErrVar(p_var: *const Variant) -> i64 in EDX:EAX (pops 4 bytes)
                let p_var = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if p_var != 0 {
                    let var = self.mem.read_variant_deref(p_var);
                    let vt = var.vt & !crate::types::variant::VT_BYREF;
                    if vt == crate::types::variant::VT_ERROR {
                        self.regs.eax = 0;
                        self.regs.edx = 0;
                        self.handle_error(13)?;
                    } else {
                        let cy_val = match vt {
                            crate::types::VT_CY => i64::from_le_bytes(var.data[0..8].try_into().unwrap()),
                            crate::types::VT_I2 => (i16::from_le_bytes([var.data[0], var.data[1]]) as i64) * 10000,
                            crate::types::VT_I4 => (i32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) as i64) * 10000,
                            crate::types::VT_UI1 => (var.data[0] as i64) * 10000,
                            crate::types::VT_R4 => crate::math::round_to_even((f32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) as f64) * 10000.0) as i64,
                            crate::types::VT_R8 | crate::types::VT_DATE => crate::math::round_to_even(f64::from_le_bytes(var.data[0..8].try_into().unwrap()) * 10000.0) as i64,
                            _ => 0,
                        };
                        self.regs.eax = cy_val as u32;
                        self.regs.edx = (cy_val as u64 >> 32) as u32;
                    }
                } else {
                    self.regs.eax = 0;
                    self.regs.edx = 0;
                }
            }
            191 => {
                // __vbaCyFix(val: i64 on stack) -> i64 in EDX:EAX (pops 8 bytes)
                let low = self.mem.read_u32(self.regs.esp);
                let high = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let val = (((high as u64) << 32) | (low as u64)) as i64;
                let res = (val / 10000) * 10000;
                self.regs.eax = res as u32;
                self.regs.edx = (res as u64 >> 32) as u32;
            }
            194 => {
                // __vbaCyI2(val: i16 on stack) -> i64 in EDX:EAX (pops 4 bytes)
                let val = (self.mem.read_u32(self.regs.esp) as i16 as i64) * 10000;
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = val as u32;
                self.regs.edx = (val as u64 >> 32) as u32;
            }
            196 => {
                // __vbaCyI4(val: i32 on stack) -> i64 in EDX:EAX (pops 4 bytes)
                let val = (self.mem.read_u32(self.regs.esp) as i32 as i64) * 10000;
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = val as u32;
                self.regs.edx = (val as u64 >> 32) as u32;
            }
            197 => {
                // __vbaCyInt(val: i64 on stack) -> i64 in EDX:EAX (pops 8 bytes)
                let low = self.mem.read_u32(self.regs.esp);
                let high = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let val = (((high as u64) << 32) | (low as u64)) as i64;
                let res = val.div_euclid(10000) * 10000;
                self.regs.eax = res as u32;
                self.regs.edx = (res as u64 >> 32) as u32;
            }
            201 => {
                // __vbaCySgn(val: i64 on stack) -> i16 in AX (pops 8 bytes)
                let low = self.mem.read_u32(self.regs.esp);
                let high = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let val = (((high as u64) << 32) | (low as u64)) as i64;
                let sgn: i16 = if val > 0 { 1 } else if val < 0 { -1 } else { 0 };
                self.regs.eax = (sgn as u16) as u32;
            }
            203 => {
                // __vbaCySub(c1: i64, c2: i64) -> i64 in EDX:EAX (pops 16 bytes: c1 at esp, c2 at esp+8)
                let c1_low = self.mem.read_u32(self.regs.esp);
                let c1_high = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let c2_low = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let c2_high = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                self.regs.esp = self.regs.esp.wrapping_add(16);
                let c1 = (((c1_high as u64) << 32) | (c1_low as u64)) as i64;
                let c2 = (((c2_high as u64) << 32) | (c2_low as u64)) as i64;
                let res = c1.wrapping_sub(c2);
                self.regs.eax = res as u32;
                self.regs.edx = (res as u64 >> 32) as u32;
            }
            204 => {
                // __vbaCyUI1(val: u8 on stack) -> i64 in EDX:EAX (pops 4 bytes)
                let val = (self.mem.read_u32(self.regs.esp) as u8 as i64) * 10000;
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = val as u32;
                self.regs.edx = (val as u64 >> 32) as u32;
            }
            207 => {
                // __vbaDateR8(val: f64 on stack) -> f64 in ST0 (pops 8 bytes)
                let low = self.mem.read_u32(self.regs.esp);
                let high = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let bits = ((high as u64) << 32) | (low as u64);
                let val = f64::from_bits(bits);
                self.fpu.push(val);
            }
            259 => {
                // __vbaI2Cy(val: i64 on stack) -> i16 in AX (pops 8 bytes)
                let low = self.mem.read_u32(self.regs.esp);
                let high = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let val = (((high as u64) << 32) | (low as u64)) as i64;
                let res = crate::math::round_to_even((val as f64) / 10000.0) as i16;
                self.regs.eax = (res as u16) as u32;
            }
            260 | 267 => {
                // __vbaI2ErrVar (260) / __vbaI4ErrVar (267) (pops 4 bytes)
                let p_var = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if p_var != 0 {
                    let var = self.mem.read_variant_deref(p_var);
                    let vt = (var.vt & 0x0FFF) & !crate::types::variant::VT_BYREF;
                    if vt == crate::types::variant::VT_ERROR {
                        self.regs.eax = 0;
                        self.handle_error(13)?;
                    } else if ordinal == 260 {
                        let i2_val = match vt {
                            crate::types::VT_I2 | crate::types::VT_BOOL => i16::from_le_bytes([var.data[0], var.data[1]]),
                            crate::types::VT_I4 => i32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) as i16,
                            crate::types::VT_UI1 => var.data[0] as i16,
                            crate::types::VT_CY => crate::math::round_to_even(i64::from_le_bytes(var.data[0..8].try_into().unwrap()) as f64 / 10000.0) as i16,
                            crate::types::VT_R4 => crate::math::round_to_even(f32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) as f64) as i16,
                            crate::types::VT_R8 | crate::types::VT_DATE => crate::math::round_to_even(f64::from_le_bytes(var.data[0..8].try_into().unwrap())) as i16,
                            _ => var.as_i32().unwrap_or(0) as i16,
                        };
                        self.regs.eax = (i2_val as u16) as u32;
                    } else {
                        let i4_val = match vt {
                            crate::types::VT_I4 => i32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]),
                            crate::types::VT_I2 | crate::types::VT_BOOL => i16::from_le_bytes([var.data[0], var.data[1]]) as i32,
                            crate::types::VT_UI1 => var.data[0] as i32,
                            crate::types::VT_CY => crate::math::round_to_even(i64::from_le_bytes(var.data[0..8].try_into().unwrap()) as f64 / 10000.0) as i32,
                            crate::types::VT_R4 => crate::math::round_to_even(f32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) as f64) as i32,
                            crate::types::VT_R8 | crate::types::VT_DATE => crate::math::round_to_even(f64::from_le_bytes(var.data[0..8].try_into().unwrap())) as i32,
                            _ => var.as_i32().unwrap_or(0),
                        };
                        self.regs.eax = i4_val as u32;
                    }
                } else {
                    self.regs.eax = 0;
                }
            }
            264 => {
                // __vbaI4Cy(val: i64 on stack) -> i32 in EAX (pops 8 bytes)
                let low = self.mem.read_u32(self.regs.esp);
                let high = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let val = (((high as u64) << 32) | (low as u64)) as i64;
                let res = crate::math::round_to_even((val as f64) / 10000.0) as i32;
                self.regs.eax = res as u32;
            }
            363 => {
                // __vbaR4Cy(val: i64 on stack) -> f32 in ST0 (pops 8 bytes)
                let low = self.mem.read_u32(self.regs.esp);
                let high = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let val = (((high as u64) << 32) | (low as u64)) as i64;
                let res = ((val as f64) / 10000.0) as f32;
                self.fpu.push(res as f64);
            }
            364 => {
                // __vbaR4ErrVar(p_var: *const Variant) -> f32 in ST0 (pops 4 bytes)
                let p_var = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if p_var != 0 {
                    let var = self.mem.read_variant_deref(p_var);
                    let vt = var.vt & !crate::types::variant::VT_BYREF;
                    if vt == crate::types::variant::VT_ERROR {
                        self.handle_error(13)?;
                    } else {
                        let f = match vt {
                            crate::types::VT_R4 => f32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]),
                            crate::types::VT_R8 | crate::types::VT_DATE => f64::from_le_bytes(var.data[0..8].try_into().unwrap()) as f32,
                            crate::types::VT_I2 => i16::from_le_bytes([var.data[0], var.data[1]]) as f32,
                            crate::types::VT_I4 => i32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) as f32,
                            crate::types::VT_UI1 => var.data[0] as f32,
                            crate::types::VT_CY => (i64::from_le_bytes(var.data[0..8].try_into().unwrap()) as f64 / 10000.0) as f32,
                            _ => 0.0,
                        };
                        self.fpu.push(f as f64);
                    }
                } else {
                    self.fpu.push(0.0);
                }
            }
            366 => {
                // __vbaR4Sgn(val: f32 on stack) -> i16 in AX (pops 4 bytes)
                let bits = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let val = f32::from_bits(bits);
                let sgn: i16 = if val > 0.0 { 1 } else if val < 0.0 { -1 } else { 0 };
                self.regs.eax = (sgn as u16) as u32;
            }
            371 => {
                // __vbaR8FixI2(val in ST0) -> i16 in AX (pops ST0)
                let val = self.fpu.pop();
                let res = val.trunc() as i16;
                self.regs.eax = (res as u16) as u32;
            }
            372 => {
                // __vbaR8FixI4(val in ST0) -> i32 in EAX (pops ST0)
                let val = self.fpu.pop();
                let res = val.trunc() as i32;
                self.regs.eax = res as u32;
            }
            374 => {
                // __vbaR8IntI2(val in ST0) -> i16 in AX (pops ST0)
                let val = self.fpu.pop();
                let res = val.floor() as i16;
                self.regs.eax = (res as u16) as u32;
            }
            375 => {
                // __vbaR8IntI4(val in ST0) -> i32 in EAX (pops ST0)
                let val = self.fpu.pop();
                let res = val.floor() as i32;
                self.regs.eax = res as u32;
            }
            376 => {
                // __vbaR8Sgn(val: f64 on stack) -> i16 in AX (pops 8 bytes)
                let low = self.mem.read_u32(self.regs.esp);
                let high = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let val = f64::from_bits(((high as u64) << 32) | (low as u64));
                let sgn: i16 = if val > 0.0 { 1 } else if val < 0.0 { -1 } else { 0 };
                self.regs.eax = (sgn as u16) as u32;
            }
            439 => {
                // __vbaUI1Cy(val: i64 on stack) -> u8 in AL (pops 8 bytes)
                let low = self.mem.read_u32(self.regs.esp);
                let high = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let val = (((high as u64) << 32) | (low as u64)) as i64;
                let res = crate::math::round_to_even((val as f64) / 10000.0) as u8;
                self.regs.eax = res as u32;
            }
            442 => {
                // __vbaUI1ErrVar(p_var: *const Variant) -> u8 in AL (pops 4 bytes)
                let p_var = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if p_var != 0 {
                    let var = self.mem.read_variant_deref(p_var);
                    let vt = var.vt & !crate::types::variant::VT_BYREF;
                    if vt == crate::types::variant::VT_ERROR {
                        self.regs.eax = 0;
                        self.handle_error(13)?;
                    } else {
                        let u8_val = match vt {
                            crate::types::VT_UI1 => var.data[0],
                            crate::types::VT_I2 => i16::from_le_bytes([var.data[0], var.data[1]]) as u8,
                            crate::types::VT_I4 => i32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) as u8,
                            crate::types::VT_CY => (i64::from_le_bytes(var.data[0..8].try_into().unwrap()) / 10000) as u8,
                            crate::types::VT_R4 => crate::math::round_to_even(f32::from_le_bytes([var.data[0], var.data[1], var.data[2], var.data[3]]) as f64) as u8,
                            crate::types::VT_R8 | crate::types::VT_DATE => crate::math::round_to_even(f64::from_le_bytes(var.data[0..8].try_into().unwrap())) as u8,
                            _ => 0,
                        };
                        self.regs.eax = u8_val as u32;
                    }
                } else {
                    self.regs.eax = 0;
                }
            }
            156 => {
                // _CIatan: ST0 = atan(ST0)
                let v = self.fpu.pop();
                self.fpu.push(v.atan());
            }
            157 => {
                // _CIcos: ST0 = cos(ST0)
                let v = self.fpu.pop();
                self.fpu.push(v.cos());
            }
            158 => {
                // _CIexp: ST0 = exp(ST0)
                let v = self.fpu.pop();
                self.fpu.push(v.exp());
            }
            159 => {
                // _CIlog: ST0 = ln(ST0)
                let v = self.fpu.pop();
                self.fpu.push(v.ln());
            }
            160 => {
                // _CIsin: ST0 = sin(ST0)
                let v = self.fpu.pop();
                self.fpu.push(v.sin());
            }
            161 => {
                // _CIsqrt: ST0 = sqrt(ST0)
                let v = self.fpu.pop();
                self.fpu.push(v.sqrt());
            }
            162 => {
                // _CItan: ST0 = tan(ST0)
                let v = self.fpu.pop();
                self.fpu.push(v.tan());
            }
            182 => {
                // __vbaChkstk(size in EAX): adjusts stack pointer by subtracting EAX from ESP
                self.regs.esp = self.regs.esp.wrapping_sub(self.regs.eax);
            }
            224 => {
                // __vbaFPException()
                self.regs.eax = 0;
            }
            354 => {
                // __vbaPowerR8(base: f64, exp: f64) (pops 16 bytes) -> ST0
                let base = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let exp = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                self.regs.esp = self.regs.esp.wrapping_add(16);
                self.fpu.push(base.powf(exp));
            }
            582 => {
                // rtcSin(val: f64) (pops 8 bytes) -> ST0
                let val = f64::from_bits(self.mem.read_u64(self.regs.esp));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.fpu.push(val.sin());
            }
            583 => {
                // rtcCos(val: f64) (pops 8 bytes) -> ST0
                let val = f64::from_bits(self.mem.read_u64(self.regs.esp));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.fpu.push(val.cos());
            }
            584 => {
                // rtcTan(val: f64) (pops 8 bytes) -> ST0
                let val = f64::from_bits(self.mem.read_u64(self.regs.esp));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.fpu.push(val.tan());
            }
            585 => {
                // rtcAtn(val: f64) (pops 8 bytes) -> ST0
                let val = f64::from_bits(self.mem.read_u64(self.regs.esp));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.fpu.push(val.atan());
            }
            586 => {
                // rtcExp(val: f64) (pops 8 bytes) -> ST0
                let val = f64::from_bits(self.mem.read_u64(self.regs.esp));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.fpu.push(val.exp());
            }
            587 => {
                // rtcLog(val: f64) (pops 8 bytes) -> ST0
                let val = f64::from_bits(self.mem.read_u64(self.regs.esp));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.fpu.push(val.ln());
            }
            614 => {
                // rtcSqr(val: f64) (pops 8 bytes) -> ST0
                let val = f64::from_bits(self.mem.read_u64(self.regs.esp));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.fpu.push(val.sqrt());
            }
            659 => {
                // _adj_fdiv_m16i(divisor_ptr: *const i16) - cdecl
                let ptr = self.mem.read_u32(self.regs.esp);
                let val = (self.mem.read_u16(ptr) as i16) as f64;
                self.fpu.st[self.fpu.top] /= val;
            }
            688 => {
                // _adj_fdiv_m32(divisor_ptr: *const f32) - cdecl
                let ptr = self.mem.read_u32(self.regs.esp);
                let val = f32::from_bits(self.mem.read_u32(ptr)) as f64;
                self.fpu.st[self.fpu.top] /= val;
            }
            701 => {
                // _adj_fdiv_m32i(divisor_ptr: *const i32) - cdecl
                let ptr = self.mem.read_u32(self.regs.esp);
                let val = (self.mem.read_u32(ptr) as i32) as f64;
                self.fpu.st[self.fpu.top] /= val;
            }
            718 => {
                // _adj_fdiv_m64(divisor_ptr: *const f64) - cdecl
                let ptr = self.mem.read_u32(self.regs.esp);
                let val = f64::from_bits(self.mem.read_u64(ptr));
                self.fpu.st[self.fpu.top] /= val;
            }
            719 => {
                // _adj_fdiv_r()
                let y = self.fpu.pop();
                self.fpu.st[self.fpu.top] /= y;
            }
            720 => {
                // _adj_fdivr_m16i(divisor_ptr: *const i16) - cdecl
                let ptr = self.mem.read_u32(self.regs.esp);
                let val = (self.mem.read_u16(ptr) as i16) as f64;
                self.fpu.st[self.fpu.top] = val / self.fpu.st[self.fpu.top];
            }
            721 => {
                // _adj_fdivr_m32(divisor_ptr: *const f32) - cdecl
                let ptr = self.mem.read_u32(self.regs.esp);
                let val = f32::from_bits(self.mem.read_u32(ptr)) as f64;
                self.fpu.st[self.fpu.top] = val / self.fpu.st[self.fpu.top];
            }
            722 => {
                // _adj_fdivr_m32i(divisor_ptr: *const i32) - cdecl
                let ptr = self.mem.read_u32(self.regs.esp);
                let val = (self.mem.read_u32(ptr) as i32) as f64;
                self.fpu.st[self.fpu.top] = val / self.fpu.st[self.fpu.top];
            }
            723 => {
                // _adj_fdivr_m64(divisor_ptr: *const f64) - cdecl
                let ptr = self.mem.read_u32(self.regs.esp);
                let val = f64::from_bits(self.mem.read_u64(ptr));
                self.fpu.st[self.fpu.top] = val / self.fpu.st[self.fpu.top];
            }
            724 => {
                // _adj_fpatan()
                self.fpu.fpatan();
            }
            725 => {
                // _adj_fprem()
                let y = self.fpu.st[self.fpu.top];
                let x = self.fpu.st[(self.fpu.top + 1) & 7];
                self.fpu.st[self.fpu.top] = y % x;
            }
            726 => {
                // _adj_fprem1()
                let y = self.fpu.st[self.fpu.top];
                let x = self.fpu.st[(self.fpu.top + 1) & 7];
                self.fpu.st[self.fpu.top] = y % x;
            }
            727 => {
                // _adj_fptan()
                let v = self.fpu.st[self.fpu.top];
                self.fpu.st[self.fpu.top] = v.tan();
                self.fpu.push(1.0);
            }
            167 => {
                // __vbaAryDestruct(rec_info: u32, pp_sa: *mut *mut SafeArray) (stdcall, pops 8 bytes)
                let _rec_info = self.mem.read_u32(self.regs.esp);
                let pp_sa = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if pp_sa != 0 {
                    self.mem.write_u32(pp_sa, 0);
                }
                self.regs.eax = 0;
            }
            143..=146 => {
                // __vbaVarCopy (143), __vbaVarDup (144), __vbaVarMove (145), __vbaVarVargNofree (146)
                // In native VB6 x86 code: dst in ECX, src in EDX. Returns dst in EAX.
                let dst = self.regs.ecx;
                let src = self.regs.edx;
                if dst != 0 && src != 0 {
                    let vt = self.mem.read_u16(src);
                    if ordinal == 144 && vt == 8 {
                        let bstr_ptr = self.mem.read_u32(src.wrapping_add(8));
                        if bstr_ptr != 0 {
                            let chars = self.mem.read_bstr(bstr_ptr);
                            let new_bstr = self.mem.alloc_bstr(&chars);
                            self.mem.write_u16(dst, 8);
                            self.mem.write_u16(dst.wrapping_add(2), 0);
                            self.mem.write_u32(dst.wrapping_add(4), 0);
                            self.mem.write_u32(dst.wrapping_add(8), new_bstr);
                            self.mem.write_u32(dst.wrapping_add(12), 0);
                        } else {
                            for i in 0..16 {
                                let b = self.mem.read_u8(src.wrapping_add(i as u32));
                                self.mem.write_u8(dst.wrapping_add(i as u32), b);
                            }
                        }
                    } else {
                        for i in 0..16 {
                            let b = self.mem.read_u8(src.wrapping_add(i as u32));
                            self.mem.write_u8(dst.wrapping_add(i as u32), b);
                        }
                    }
                }
                self.regs.eax = dst;
            }
            140 => {
                // __vbaUI1I2(val in ecx) -> al
                let val = self.regs.ecx as i16;
                self.regs.eax = (val as u8) as u32;
            }
            141 => {
                // __vbaUI1I4(val in ecx) -> al
                let val = self.regs.ecx as i32;
                self.regs.eax = (val as u8) as u32;
            }
            134 => {
                // __vbaI2I4(val in ecx) -> ax
                let val = self.regs.ecx as i32;
                self.regs.eax = (val as i16 as u16) as u32;
            }
            131 => {
                // __vbaFreeVar(p_var in ecx)
                let p = self.regs.ecx;
                if p != 0 {
                    self.mem.write_u16(p, 0);
                }
                self.regs.eax = 0;
            }
            243 => {
                // __vbaFreeObjList(count: i32, ...) - __cdecl, caller cleans stack
                let count = self.mem.read_u32(self.regs.esp) as usize;
                for i in 0..count {
                    let p = self.mem.read_u32(self.regs.esp.wrapping_add(4 + (i as u32 * 4)));
                    if p != 0 {
                        self.mem.write_u32(p, 0);
                    }
                }
                self.regs.eax = 0;
            }
            245 => {
                // __vbaFreeVarList(count: i32, ...) - __cdecl, caller cleans stack
                let count = self.mem.read_u32(self.regs.esp) as usize;
                for i in 0..count {
                    let p = self.mem.read_u32(self.regs.esp.wrapping_add(4 + (i as u32 * 4)));
                    if p != 0 {
                        self.mem.write_u16(p, 0);
                    }
                }
                self.regs.eax = 0;
            }
            129 => {
                // __vbaFreeObj(p_obj in ecx)
                let p = self.regs.ecx;
                if p != 0 {
                    self.mem.write_u32(p, 0);
                }
                self.regs.eax = 0;
            }
            164 => {
                // __vbaAryConstruct2(p_sa: u32, desc: u32, elems: u32) (stdcall 3 args, pops 12 bytes)
                let p_sa = self.mem.read_u32(self.regs.esp);
                let desc = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let elems = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                if p_sa != 0 && desc != 0 {
                    let c_dims = self.mem.read_u16(desc);
                    let f_features = self.mem.read_u16(desc.wrapping_add(2));
                    let cb_elements = self.mem.read_u32(desc.wrapping_add(4));
                    let total_bytes = (elems as usize) * (cb_elements as usize);
                    let pv_data = self.mem.alloc_bytes(total_bytes);
                    self.mem.write_u16(p_sa, c_dims);
                    self.mem.write_u16(p_sa.wrapping_add(2), f_features);
                    self.mem.write_u32(p_sa.wrapping_add(4), cb_elements);
                    self.mem.write_u32(p_sa.wrapping_add(8), 0); // locks
                    self.mem.write_u32(p_sa.wrapping_add(12), pv_data);
                    self.mem.write_u32(p_sa.wrapping_add(16), elems);
                    let lbound = self.mem.read_i32(desc.wrapping_add(20));
                    self.mem.write_i32(p_sa.wrapping_add(20), lbound);
                }
                self.regs.eax = 0;
            }
            165 => {
                // __vbaAryConstruct(p_sa: u32, desc: u32) (stdcall 2 args, pops 8 bytes)
                let p_sa = self.mem.read_u32(self.regs.esp);
                let desc = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if p_sa != 0 && desc != 0 {
                    let c_dims = self.mem.read_u16(desc);
                    let f_features = self.mem.read_u16(desc.wrapping_add(2));
                    let cb_elements = self.mem.read_u32(desc.wrapping_add(4));
                    let c_elements = self.mem.read_u32(desc.wrapping_add(16));
                    let total_bytes = (c_elements as usize) * (cb_elements as usize);
                    let pv_data = self.mem.alloc_bytes(total_bytes);
                    self.mem.write_u16(p_sa, c_dims);
                    self.mem.write_u16(p_sa.wrapping_add(2), f_features);
                    self.mem.write_u32(p_sa.wrapping_add(4), cb_elements);
                    self.mem.write_u32(p_sa.wrapping_add(8), 0);
                    self.mem.write_u32(p_sa.wrapping_add(12), pv_data);
                    self.mem.write_u32(p_sa.wrapping_add(16), c_elements);
                    let lbound = self.mem.read_i32(desc.wrapping_add(20));
                    self.mem.write_i32(p_sa.wrapping_add(20), lbound);
                }
                self.regs.eax = 0;
            }
            671 => {
                // rtcSLN(cost: f64, salvage: f64, life: f64) -> f64 (pops 24 bytes)
                let cost = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let salvage = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                let life = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(16)));
                self.regs.esp = self.regs.esp.wrapping_add(24);
                let res = unsafe { crate::rtc::extended::rtcSLN(cost, salvage, life) };
                self.fpu.push(res);
            }
            672 => {
                // rtcSYD(cost: f64, salvage: f64, life: f64, period: f64) -> f64 (pops 32 bytes)
                let cost = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let salvage = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                let life = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(16)));
                let period = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(24)));
                self.regs.esp = self.regs.esp.wrapping_add(32);
                let res = unsafe { crate::rtc::extended::rtcSYD(cost, salvage, life, period) };
                self.fpu.push(res);
            }
            673 => {
                // rtcDDB(cost, salvage, life, period, factor) -> f64 (pops 36 bytes)
                let cost = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let salvage = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                let life = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(16)));
                let period = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(24)));
                let factor_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(32));
                self.regs.esp = self.regs.esp.wrapping_add(36);
                let factor_var = if factor_ptr != 0 { self.mem.read_variant(factor_ptr) } else { crate::types::Variant::empty() };
                let res = unsafe { crate::rtc::extended::rtcDDB(cost, salvage, life, period, &factor_var) };
                self.fpu.push(res);
            }
            674 => {
                // rtcIPMT(rate, per, nper, pv, fv, type) -> f64 (pops 40 bytes)
                let rate = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let per = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                let nper = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(16)));
                let pv = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(24)));
                let fv_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(32));
                let type_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(36));
                self.regs.esp = self.regs.esp.wrapping_add(40);
                let fv_var = if fv_ptr != 0 { self.mem.read_variant(fv_ptr) } else { crate::types::Variant::empty() };
                let type_var = if type_ptr != 0 { self.mem.read_variant(type_ptr) } else { crate::types::Variant::empty() };
                let res = unsafe { crate::rtc::extended::rtcIPMT(rate, per, nper, pv, &fv_var, &type_var) };
                self.fpu.push(res);
            }
            675 => {
                // rtcPPMT(rate, per, nper, pv, fv, type) -> f64 (pops 40 bytes)
                let rate = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let per = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                let nper = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(16)));
                let pv = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(24)));
                let fv_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(32));
                let type_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(36));
                self.regs.esp = self.regs.esp.wrapping_add(40);
                let fv_var = if fv_ptr != 0 { self.mem.read_variant(fv_ptr) } else { crate::types::Variant::empty() };
                let type_var = if type_ptr != 0 { self.mem.read_variant(type_ptr) } else { crate::types::Variant::empty() };
                let res = unsafe { crate::rtc::extended::rtcPPMT(rate, per, nper, pv, &fv_var, &type_var) };
                self.fpu.push(res);
            }
            676 => {
                // rtcPMT(rate, nper, pv, fv, type) -> f64 (pops 32 bytes)
                let rate = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let nper = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                let pv = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(16)));
                let fv_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(24));
                let type_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(28));
                self.regs.esp = self.regs.esp.wrapping_add(32);
                let fv_var = if fv_ptr != 0 { self.mem.read_variant(fv_ptr) } else { crate::types::Variant::empty() };
                let type_var = if type_ptr != 0 { self.mem.read_variant(type_ptr) } else { crate::types::Variant::empty() };
                let res = unsafe { crate::rtc::extended::rtcPMT(rate, nper, pv, &fv_var, &type_var) };
                self.fpu.push(res);
            }
            677 => {
                // rtcPV(rate, nper, pmt, fv, type) -> f64 (pops 32 bytes)
                let rate = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let nper = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                let pmt = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(16)));
                let fv_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(24));
                let type_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(28));
                self.regs.esp = self.regs.esp.wrapping_add(32);
                let fv_var = if fv_ptr != 0 { self.mem.read_variant(fv_ptr) } else { crate::types::Variant::empty() };
                let type_var = if type_ptr != 0 { self.mem.read_variant(type_ptr) } else { crate::types::Variant::empty() };
                let res = unsafe { crate::rtc::extended::rtcPV(rate, nper, pmt, &fv_var, &type_var) };
                self.fpu.push(res);
            }
            678 => {
                // rtcFV(rate, nper, pmt, pv, type) -> f64 (pops 32 bytes)
                let rate = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let nper = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                let pmt = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(16)));
                let pv_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(24));
                let type_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(28));
                self.regs.esp = self.regs.esp.wrapping_add(32);
                let pv_var = if pv_ptr != 0 { self.mem.read_variant(pv_ptr) } else { crate::types::Variant::empty() };
                let type_var = if type_ptr != 0 { self.mem.read_variant(type_ptr) } else { crate::types::Variant::empty() };
                let res = unsafe { crate::rtc::extended::rtcFV(rate, nper, pmt, &pv_var, &type_var) };
                self.fpu.push(res);
            }
            679 => {
                // rtcNPer(rate, pmt, pv, fv, type) -> f64 (pops 32 bytes)
                let rate = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let pmt = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                let pv = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(16)));
                let fv_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(24));
                let type_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(28));
                self.regs.esp = self.regs.esp.wrapping_add(32);
                let fv_var = if fv_ptr != 0 { self.mem.read_variant(fv_ptr) } else { crate::types::Variant::empty() };
                let type_var = if type_ptr != 0 { self.mem.read_variant(type_ptr) } else { crate::types::Variant::empty() };
                let res = unsafe { crate::rtc::extended::rtcNPer(rate, pmt, pv, &fv_var, &type_var) };
                self.fpu.push(res);
            }
            680 => {
                // rtcRate(nper, pmt, pv, fv, type, guess) -> f64 (pops 36 bytes)
                let nper = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let pmt = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                let pv = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(16)));
                let fv_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(24));
                let type_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(28));
                let guess_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(32));
                self.regs.esp = self.regs.esp.wrapping_add(36);
                let fv_var = if fv_ptr != 0 { self.mem.read_variant(fv_ptr) } else { crate::types::Variant::empty() };
                let type_var = if type_ptr != 0 { self.mem.read_variant(type_ptr) } else { crate::types::Variant::empty() };
                let guess_var = if guess_ptr != 0 { self.mem.read_variant(guess_ptr) } else { crate::types::Variant::empty() };
                let res = unsafe { crate::rtc::extended::rtcRate(nper, pmt, pv, &fv_var, &type_var, &guess_var) };
                self.fpu.push(res);
            }
            684 => {
                // rtcNPV(rate: f64, pp_values: *const *mut SafeArray) -> f64 (pops 12 bytes)
                let rate = f64::from_bits(self.mem.read_u64(self.regs.esp));
                let pp_values = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let p_sa = self.mem.read_u32(pp_values);
                let pv_data = self.mem.read_u32(p_sa.wrapping_add(12));
                let count = self.mem.read_u32(p_sa.wrapping_add(16)) as usize;
                let mut npv = 0.0;
                for i in 0..count {
                    let v = f64::from_bits(self.mem.read_u64(pv_data.wrapping_add(i as u32 * 8)));
                    npv += v / (1.0 + rate).powi((i + 1) as i32);
                }
                self.fpu.push(npv);
            }
            682 => {
                // rtcIRR(pp_values: *const *mut SafeArray, guess: *const Variant) -> f64 (pops 8 bytes)
                let pp_values = self.mem.read_u32(self.regs.esp);
                let guess_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let p_sa = self.mem.read_u32(pp_values);
                let pv_data = self.mem.read_u32(p_sa.wrapping_add(12));
                let count = self.mem.read_u32(p_sa.wrapping_add(16)) as usize;
                let mut flows = Vec::with_capacity(count);
                for i in 0..count {
                    flows.push(f64::from_bits(self.mem.read_u64(pv_data.wrapping_add(i as u32 * 8))));
                }
                let mut sa_host = crate::types::SafeArray {
                    c_dims: 1,
                    f_features: 0x0092,
                    cb_elements: 8,
                    c_locks: 0,
                    pv_data: flows.as_ptr() as *mut core::ffi::c_void,
                    rgsabound: [crate::types::SafeArrayBound { c_elements: count as u32, l_lbound: 0 }],
                };
                let guess_var = if guess_ptr != 0 { self.mem.read_variant(guess_ptr) } else { crate::types::Variant::empty() };
                let res = unsafe { crate::rtc::extended::rtcIRR(&mut sa_host, if guess_ptr != 0 { &guess_var } else { core::ptr::null() }) };
                self.fpu.push(res);
            }
            683 => {
                // rtcMIRR(pp_values: *const *mut SafeArray, fin_rate: f64, rein_rate: f64) -> f64 (pops 20 bytes)
                let pp_values = self.mem.read_u32(self.regs.esp);
                let fin_rate = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(4)));
                let rein_rate = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(12)));
                self.regs.esp = self.regs.esp.wrapping_add(20);
                let p_sa = self.mem.read_u32(pp_values);
                let pv_data = self.mem.read_u32(p_sa.wrapping_add(12));
                let count = self.mem.read_u32(p_sa.wrapping_add(16)) as usize;
                let mut flows = Vec::with_capacity(count);
                for i in 0..count {
                    flows.push(f64::from_bits(self.mem.read_u64(pv_data.wrapping_add(i as u32 * 8))));
                }
                let mut sa_host = crate::types::SafeArray {
                    c_dims: 1,
                    f_features: 0x0092,
                    cb_elements: 8,
                    c_locks: 0,
                    pv_data: flows.as_ptr() as *mut core::ffi::c_void,
                    rgsabound: [crate::types::SafeArrayBound { c_elements: count as u32, l_lbound: 0 }],
                };
                let res = unsafe { crate::rtc::extended::rtcMIRR(&mut sa_host, fin_rate, rein_rate) };
                self.fpu.push(res);
            }
            277 => {
                // __vbaInStr(compare: i32, needle: Bstr, haystack: Bstr, start: i32) -> i32 (stdcall, pops 16 bytes)
                let compare = self.mem.read_u32(self.regs.esp) as i32;
                let needle_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let hay_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let start = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(16);
                let needle_str = self.mem.read_bstr_string(needle_ptr);
                let hay_str = self.mem.read_bstr_string(hay_ptr);
                let start_idx = (start.max(1) - 1) as usize;
                let found = if start_idx < hay_str.len() {
                    let hay_sub = &hay_str[start_idx..];
                    if compare == 1 {
                        let hay_lower = hay_sub.to_lowercase();
                        let needle_lower = needle_str.to_lowercase();
                        hay_lower.find(&needle_lower).map(|i| (start_idx + i + 1) as u32)
                    } else {
                        hay_sub.find(&needle_str).map(|i| (start_idx + i + 1) as u32)
                    }
                } else {
                    None
                };
                self.regs.eax = found.unwrap_or(0);
            }
            689 => {
                // rtcGetSetting(appname, section, key, def_var: Variant) -> Bstr (pops 28 bytes)
                let app_ptr = self.mem.read_u32(self.regs.esp);
                let sec_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let key_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let def_vt = self.mem.read_u16(self.regs.esp.wrapping_add(12));
                let def_val = if def_vt == crate::types::VT_BSTR {
                    let bstr_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(20));
                    self.mem.read_bstr_string(bstr_ptr)
                } else {
                    String::new()
                };
                self.regs.esp = self.regs.esp.wrapping_add(28);
                let app = self.mem.read_bstr_string(app_ptr);
                let sec = self.mem.read_bstr_string(sec_ptr);
                let key = self.mem.read_bstr_string(key_ptr);
                let reg_key = format!("software\\vb and vba program settings\\{}\\{}\\{}", app, sec, key);
                let val = crate::registry::reg_read(&reg_key).unwrap_or(def_val);
                let res = self.mem.alloc_bstr_from_str(&val);
                self.regs.eax = res;
            }
            690 => {
                // rtcSaveSetting(appname, section, key, setting) (pops 16 bytes)
                let app_ptr = self.mem.read_u32(self.regs.esp);
                let sec_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let key_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let set_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                self.regs.esp = self.regs.esp.wrapping_add(16);
                let app = self.mem.read_bstr_string(app_ptr);
                let sec = self.mem.read_bstr_string(sec_ptr);
                let key = self.mem.read_bstr_string(key_ptr);
                let val = self.mem.read_bstr_string(set_ptr);
                let reg_key = format!("software\\vb and vba program settings\\{}\\{}\\{}", app, sec, key);
                crate::registry::reg_write(&reg_key, &val);
                self.regs.eax = 0;
            }
            450 => {
                // __vbaVarAdd(dst: *mut Variant, right: *const Variant, left: *const Variant) -> *mut Variant (pops 12 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let right_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let left_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let v_right = self.read_variant_for_host(right_ptr);
                let v_left = self.read_variant_for_host(left_ptr);
                let mut out = crate::types::Variant::empty();
                unsafe {
                    crate::vba::__vbaVarAdd(&mut out, &v_right, &v_left);
                }
                self.write_host_variant(dst_ptr, &out);
                self.regs.eax = dst_ptr;
            }
            453..=458 | 489..=494 => {
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let right_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let left_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let v_left = self.mem.read_variant_deref(left_ptr);
                let v_right = self.mem.read_variant_deref(right_ptr);
                let text_cmp = ordinal >= 489;
                let ord = self.compare_variants(&v_left, &v_right, text_cmp);
                let is_true = match ordinal {
                    453 | 489 => ord == std::cmp::Ordering::Equal,
                    454 | 490 => ord == std::cmp::Ordering::Greater || ord == std::cmp::Ordering::Equal,
                    455 | 491 => ord == std::cmp::Ordering::Greater,
                    456 | 492 => ord == std::cmp::Ordering::Less || ord == std::cmp::Ordering::Equal,
                    457 | 493 => ord == std::cmp::Ordering::Less,
                    458 | 494 => ord != std::cmp::Ordering::Equal,
                    _ => false,
                };
                let out = crate::types::Variant::from_bool(is_true);
                self.mem.write_variant(dst_ptr, &out);
                self.regs.eax = dst_ptr;
            }
            497..=508 => {
                let right_ptr = self.mem.read_u32(self.regs.esp);
                let left_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let v_left = self.mem.read_variant_deref(left_ptr);
                let v_right = self.mem.read_variant_deref(right_ptr);
                let text_cmp = ordinal <= 502;
                let ord = self.compare_variants(&v_left, &v_right, text_cmp);
                let is_true = match ordinal {
                    497 | 503 => ord == std::cmp::Ordering::Equal,
                    498 | 504 => ord == std::cmp::Ordering::Greater || ord == std::cmp::Ordering::Equal,
                    499 | 505 => ord == std::cmp::Ordering::Greater,
                    500 | 506 => ord == std::cmp::Ordering::Less || ord == std::cmp::Ordering::Equal,
                    501 | 507 => ord == std::cmp::Ordering::Less,
                    502 | 508 => ord != std::cmp::Ordering::Equal,
                    _ => false,
                };
                self.regs.eax = if is_true { 0xFFFF } else { 0 };
            }
            479 => {
                // __vbaVarNot(dst, src) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let v = self.read_variant_for_host(src_ptr);
                let mut out = crate::types::Variant::empty();
                unsafe {
                    crate::vba::__vbaVarNot(&mut out, &v);
                }
                self.write_host_variant(dst_ptr, &out);
                self.regs.eax = dst_ptr;
            }
            449 => {
                // __vbaVarAbs(dst, src) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let v = self.read_variant_for_host(src_ptr);
                let mut out = crate::types::Variant::empty();
                unsafe {
                    crate::vba::__vbaVarAbs(&mut out, &v);
                }
                self.write_host_variant(dst_ptr, &out);
                self.regs.eax = dst_ptr;
            }
            451 | 460 | 461 | 466 | 467 | 476 | 477 | 480 | 481 | 488 | 509 => {
                // Binary variant ops: __vbaVarAnd (451), __vbaVarDiv (460), __vbaVarEqv (461),
                // __vbaVarIdiv (466), __vbaVarImp (467), __vbaVarMod (476), __vbaVarMul (477),
                // __vbaVarOr (480), __vbaVarPow (481), __vbaVarSub (488), __vbaVarXor (509)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let right_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let left_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let v_right = self.read_variant_for_host(right_ptr);
                let v_left = self.read_variant_for_host(left_ptr);
                let mut out = crate::types::Variant::empty();
                unsafe {
                    match ordinal {
                        451 => { crate::vba::__vbaVarAnd(&mut out, &v_right, &v_left); }
                        460 => { crate::vba::__vbaVarDiv(&mut out, &v_right, &v_left); }
                        461 => { crate::vba::__vbaVarEqv(&mut out, &v_right, &v_left); }
                        466 => { crate::vba::__vbaVarIdiv(&mut out, &v_right, &v_left); }
                        467 => { crate::vba::__vbaVarImp(&mut out, &v_left, &v_right); }
                        476 => { crate::vba::__vbaVarMod(&mut out, &v_right, &v_left); }
                        477 => { crate::vba::__vbaVarMul(&mut out, &v_right, &v_left); }
                        480 => { crate::vba::__vbaVarOr(&mut out, &v_right, &v_left); }
                        481 => { crate::vba::__vbaVarPow(&mut out, &v_right, &v_left); }
                        488 => { crate::vba::__vbaVarSub(&mut out, &v_right, &v_left); }
                        509 => { crate::vba::__vbaVarXor(&mut out, &v_right, &v_left); }
                        _ => {}
                    }
                }
                self.write_host_variant(dst_ptr, &out);
                self.regs.eax = dst_ptr;
            }
            691 => {
                // rtcDeleteSetting(app: Bstr, sec: Variant, key: Variant) (pops 36 bytes)
                let app_ptr = self.mem.read_u32(self.regs.esp);
                let app = self.mem.read_bstr_string(app_ptr);
                let sec_vt = self.mem.read_u16(self.regs.esp.wrapping_add(4));
                let sec = if sec_vt == crate::types::VT_BSTR {
                    let b = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                    Some(self.mem.read_bstr_string(b))
                } else {
                    None
                };
                let key_vt = self.mem.read_u16(self.regs.esp.wrapping_add(20));
                let key = if key_vt == crate::types::VT_BSTR {
                    let b = self.mem.read_u32(self.regs.esp.wrapping_add(28));
                    Some(self.mem.read_bstr_string(b))
                } else {
                    None
                };
                self.regs.esp = self.regs.esp.wrapping_add(36);
                match (sec.as_deref(), key.as_deref()) {
                    (Some(s), Some(k)) if !k.is_empty() => {
                        let reg_key = format!("software\\vb and vba program settings\\{}\\{}\\{}", app, s, k);
                        crate::registry::reg_delete(&reg_key);
                    }
                    (Some(s), _) if !s.is_empty() => {
                        let prefix = format!("software\\vb and vba program settings\\{}\\{}\\", app, s);
                        crate::registry::reg_delete_prefix(&prefix);
                    }
                    _ => {
                        let prefix = format!("software\\vb and vba program settings\\{}\\", app);
                        crate::registry::reg_delete_prefix(&prefix);
                    }
                }
                self.regs.eax = 0;
            }
            692 => {
                // rtcGetAllSettings(dst: *mut Variant, app: Bstr, sec: Bstr) -> *mut Variant (pops 12 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let app_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let sec_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let app = self.mem.read_bstr_string(app_ptr);
                let sec = self.mem.read_bstr_string(sec_ptr);
                let prefix = format!("software\\vb and vba program settings\\{}\\{}\\", app, sec);
                let pairs = crate::registry::reg_get_prefix(&prefix);
                if pairs.is_empty() {
                    let empty_var = crate::types::Variant::empty();
                    self.mem.write_variant(dst_ptr, &empty_var);
                } else {
                    let rows = pairs.len() as u32;
                    let total_elems = rows * 2;
                    let elem_size = 16u32;
                    let sa_ptr = self.mem.alloc_bytes(32);
                    let data_ptr = self.mem.alloc_bytes((total_elems * elem_size) as usize);
                    self.mem.write_u16(sa_ptr, 2);
                    self.mem.write_u16(sa_ptr + 2, 0x0880);
                    self.mem.write_u32(sa_ptr + 4, elem_size);
                    self.mem.write_u32(sa_ptr + 8, 0);
                    self.mem.write_u32(sa_ptr + 12, data_ptr);
                    self.mem.write_u32(sa_ptr + 16, 2);
                    self.mem.write_i32(sa_ptr + 20, 0);
                    self.mem.write_u32(sa_ptr + 24, rows);
                    self.mem.write_i32(sa_ptr + 28, 0);
                    for (r, (k, val)) in pairs.iter().enumerate() {
                        let k_bstr = self.mem.alloc_bstr_from_str(k);
                        let v_bstr = self.mem.alloc_bstr_from_str(val);
                        let mut v_k = crate::types::Variant::empty();
                        v_k.vt = crate::types::VT_BSTR;
                        v_k.data[..4].copy_from_slice(&k_bstr.to_le_bytes());
                        let mut v_v = crate::types::Variant::empty();
                        v_v.vt = crate::types::VT_BSTR;
                        v_v.data[..4].copy_from_slice(&v_bstr.to_le_bytes());
                        self.mem.write_variant(data_ptr + ((r as u32 * 2) * elem_size), &v_k);
                        self.mem.write_variant(data_ptr + ((r as u32 * 2 + 1) * elem_size), &v_v);
                    }
                    let mut res_var = crate::types::Variant::empty();
                    res_var.vt = crate::types::variant::VT_ARRAY | crate::types::VT_VARIANT;
                    res_var.data[..4].copy_from_slice(&sa_ptr.to_le_bytes());
                    self.mem.write_variant(dst_ptr, &res_var);
                }
                self.regs.eax = dst_ptr;
            }
            702 => {
                // rtcFormatNumber(expression, numdigits, incl, parens, group) -> Bstr (pops 20 bytes)
                let expr_ptr = self.mem.read_u32(self.regs.esp);
                let numdigits = self.mem.read_u32(self.regs.esp.wrapping_add(4)) as i32;
                let includeleading = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as i32;
                let useparens = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                let groupdigits = self.mem.read_u32(self.regs.esp.wrapping_add(16)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(20);
                let expr_var = self.mem.read_variant(expr_ptr);
                let bstr = unsafe { crate::rtc::extended::rtcFormatNumber(&expr_var, numdigits, includeleading, useparens, groupdigits) };
                let s = unsafe { crate::types::bstr::bstr_to_string(bstr) };
                unsafe { crate::types::bstr::SysFreeString(bstr) };
                let res = self.mem.alloc_bstr_from_str(&s);
                self.regs.eax = res;
            }
            703 => {
                // rtcFormatCurrency (pops 20 bytes)
                let expr_ptr = self.mem.read_u32(self.regs.esp);
                let numdigits = self.mem.read_u32(self.regs.esp.wrapping_add(4)) as i32;
                let includeleading = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as i32;
                let useparens = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                let groupdigits = self.mem.read_u32(self.regs.esp.wrapping_add(16)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(20);
                let expr_var = self.mem.read_variant(expr_ptr);
                let bstr = unsafe { crate::rtc::extended::rtcFormatCurrency(&expr_var, numdigits, includeleading, useparens, groupdigits) };
                let s = unsafe { crate::types::bstr::bstr_to_string(bstr) };
                unsafe { crate::types::bstr::SysFreeString(bstr) };
                let res = self.mem.alloc_bstr_from_str(&s);
                self.regs.eax = res;
            }
            704 => {
                // rtcFormatPercent (pops 20 bytes)
                let expr_ptr = self.mem.read_u32(self.regs.esp);
                let numdigits = self.mem.read_u32(self.regs.esp.wrapping_add(4)) as i32;
                let includeleading = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as i32;
                let useparens = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                let groupdigits = self.mem.read_u32(self.regs.esp.wrapping_add(16)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(20);
                let expr_var = self.mem.read_variant(expr_ptr);
                let bstr = unsafe { crate::rtc::extended::rtcFormatPercent(&expr_var, numdigits, includeleading, useparens, groupdigits) };
                let s = unsafe { crate::types::bstr::bstr_to_string(bstr) };
                unsafe { crate::types::bstr::SysFreeString(bstr) };
                let res = self.mem.alloc_bstr_from_str(&s);
                self.regs.eax = res;
            }
            705 => {
                // rtcFormatDateTime (pops 8 bytes)
                let expr_ptr = self.mem.read_u32(self.regs.esp);
                let namedformat = self.mem.read_u32(self.regs.esp.wrapping_add(4)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let expr_var = self.mem.read_variant_deref(expr_ptr);
                let bstr = unsafe { crate::rtc::extended::rtcFormatDateTime(&expr_var, namedformat) };
                let s = unsafe { crate::types::bstr::bstr_to_string(bstr) };
                unsafe { crate::types::bstr::SysFreeString(bstr) };
                let res = self.mem.alloc_bstr_from_str(&s);
                self.regs.eax = res;
            }
            706 => {
                // rtcWeekdayName(weekday: i32, abbreviate: i32, firstdayofweek: i32) -> Bstr (pops 12 bytes)
                let weekday = self.mem.read_u32(self.regs.esp) as i32;
                let abbreviate = self.mem.read_u32(self.regs.esp.wrapping_add(4)) as i32;
                let firstdayofweek = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let bstr = unsafe { crate::rtc::extended::rtcWeekdayName(weekday, abbreviate, firstdayofweek) };
                let s = unsafe { crate::types::bstr::bstr_to_string(bstr) };
                unsafe { crate::types::bstr::SysFreeString(bstr) };
                let res = self.mem.alloc_bstr_from_str(&s);
                self.regs.eax = res;
            }
            707 => {
                // rtcMonthName(month: i32, abbreviate: i32) -> Bstr (pops 8 bytes)
                let month = self.mem.read_u32(self.regs.esp) as i32;
                let abbreviate = self.mem.read_u32(self.regs.esp.wrapping_add(4)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let bstr = unsafe { crate::rtc::extended::rtcMonthName(month, abbreviate) };
                let s = unsafe { crate::types::bstr::bstr_to_string(bstr) };
                unsafe { crate::types::bstr::SysFreeString(bstr) };
                let res = self.mem.alloc_bstr_from_str(&s);
                self.regs.eax = res;
            }
            538 => {
                // rtcPackDate(dst: *mut Variant, year: i32, month: i32, day: i32) -> *mut Variant (pops 16 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let year = self.mem.read_u32(self.regs.esp.wrapping_add(4)) as i32;
                let month = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as i32;
                let day = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(16);
                let serial = crate::rtc::extended::civil_ymd_to_serial_days(year, month, day);
                let mut var = crate::types::Variant::empty();
                var.vt = crate::types::variant::VT_DATE;
                var.data[..8].copy_from_slice(&serial.to_ne_bytes());
                self.mem.write_variant(dst_ptr, &var);
                self.regs.eax = dst_ptr;
            }
            539 => {
                // rtcPackTime(dst: *mut Variant, hour: i32, minute: i32, second: i32) -> *mut Variant (pops 16 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let hour = self.mem.read_u32(self.regs.esp.wrapping_add(4)) as i32;
                let minute = self.mem.read_u32(self.regs.esp.wrapping_add(8)) as i32;
                let second = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(16);
                let frac = (hour as f64 * 3600.0 + minute as f64 * 60.0 + second as f64) / 86400.0;
                let mut var = crate::types::Variant::empty();
                var.vt = crate::types::variant::VT_DATE;
                var.data[..8].copy_from_slice(&frac.to_ne_bytes());
                self.mem.write_variant(dst_ptr, &var);
                self.regs.eax = dst_ptr;
            }
            540 => {
                // rtcGetDateValue(dst: *mut Variant, str_val: Bstr) -> *mut Variant (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let str_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let s = self.mem.read_bstr_string(str_ptr);
                let bstr = crate::types::bstr::sys_alloc_string(&s);
                let mut out_var = crate::types::Variant::empty();
                unsafe {
                    crate::rtc::extended::rtcGetDateValue(&mut out_var, bstr);
                    crate::types::bstr::SysFreeString(bstr);
                }
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            541 => {
                // rtcGetTimeValue(dst: *mut Variant, str_val: Bstr) -> *mut Variant (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let str_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let s = self.mem.read_bstr_string(str_ptr);
                let bstr = crate::types::bstr::sys_alloc_string(&s);
                let mut out_var = crate::types::Variant::empty();
                unsafe {
                    crate::rtc::extended::rtcGetTimeValue(&mut out_var, bstr);
                    crate::types::bstr::SysFreeString(bstr);
                }
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            542 => {
                // rtcGetDayOfMonth(dst: *mut Variant, src: *const Variant) -> dst (stdcall, pops 8 bytes)
                // Returns a Variant (VT_I2) written into `dst`.
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let var_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let mut var = self.mem.read_variant_deref(var_ptr);
                var.vt &= 0x0FFF;
                if var.vt == crate::types::VT_BSTR {
                    let b = u32::from_le_bytes(var.data[..4].try_into().unwrap());
                    let txt = self.mem.read_bstr_string(b);
                    var = crate::rtc::extended::parse_ole_date_str(&txt)
                        .map(crate::types::Variant::date)
                        .unwrap_or_else(crate::types::Variant::empty);
                }
                if var.vt == crate::types::VT_NULL {
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::null());
                } else {
                    let res = unsafe { crate::rtc::extended::rtcGetDayOfMonth(&var) };
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::from_i16(res as i16));
                }
                self.regs.eax = dst_ptr;
            }
            543 => {
                // rtcGetHourOfDay(dst: *mut Variant, src: *const Variant) -> dst (stdcall, pops 8 bytes)
                // Returns a Variant (VT_I2) written into `dst`.
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let var_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let mut var = self.mem.read_variant_deref(var_ptr);
                var.vt &= 0x0FFF;
                if var.vt == crate::types::VT_BSTR {
                    let b = u32::from_le_bytes(var.data[..4].try_into().unwrap());
                    let txt = self.mem.read_bstr_string(b);
                    var = crate::rtc::extended::parse_ole_date_str(&txt)
                        .map(crate::types::Variant::date)
                        .unwrap_or_else(crate::types::Variant::empty);
                }
                if var.vt == crate::types::VT_NULL {
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::null());
                } else {
                    let res = unsafe { crate::rtc::extended::rtcGetHourOfDay(&var) };
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::from_i16(res as i16));
                }
                self.regs.eax = dst_ptr;
            }
            544 => {
                // rtcGetMinuteOfHour(dst: *mut Variant, src: *const Variant) -> dst (stdcall, pops 8 bytes)
                // Returns a Variant (VT_I2) written into `dst`.
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let var_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let mut var = self.mem.read_variant_deref(var_ptr);
                var.vt &= 0x0FFF;
                if var.vt == crate::types::VT_BSTR {
                    let b = u32::from_le_bytes(var.data[..4].try_into().unwrap());
                    let txt = self.mem.read_bstr_string(b);
                    var = crate::rtc::extended::parse_ole_date_str(&txt)
                        .map(crate::types::Variant::date)
                        .unwrap_or_else(crate::types::Variant::empty);
                }
                if var.vt == crate::types::VT_NULL {
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::null());
                } else {
                    let res = unsafe { crate::rtc::extended::rtcGetMinuteOfHour(&var) };
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::from_i16(res as i16));
                }
                self.regs.eax = dst_ptr;
            }
            545 => {
                // rtcGetMonthOfYear(dst: *mut Variant, src: *const Variant) -> dst (stdcall, pops 8 bytes)
                // Returns a Variant (VT_I2) written into `dst`.
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let var_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let mut var = self.mem.read_variant_deref(var_ptr);
                var.vt &= 0x0FFF;
                if var.vt == crate::types::VT_BSTR {
                    let b = u32::from_le_bytes(var.data[..4].try_into().unwrap());
                    let txt = self.mem.read_bstr_string(b);
                    var = crate::rtc::extended::parse_ole_date_str(&txt)
                        .map(crate::types::Variant::date)
                        .unwrap_or_else(crate::types::Variant::empty);
                }
                if var.vt == crate::types::VT_NULL {
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::null());
                } else {
                    let res = unsafe { crate::rtc::extended::rtcGetMonthOfYear(&var) };
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::from_i16(res as i16));
                }
                self.regs.eax = dst_ptr;
            }
            547 => {
                // rtcGetSecondOfMinute(dst: *mut Variant, src: *const Variant) -> dst (stdcall, pops 8 bytes)
                // Returns a Variant (VT_I2) written into `dst`.
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let var_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let mut var = self.mem.read_variant_deref(var_ptr);
                var.vt &= 0x0FFF;
                if var.vt == crate::types::VT_BSTR {
                    let b = u32::from_le_bytes(var.data[..4].try_into().unwrap());
                    let txt = self.mem.read_bstr_string(b);
                    var = crate::rtc::extended::parse_ole_date_str(&txt)
                        .map(crate::types::Variant::date)
                        .unwrap_or_else(crate::types::Variant::empty);
                }
                if var.vt == crate::types::VT_NULL {
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::null());
                } else {
                    let res = unsafe { crate::rtc::extended::rtcGetSecondOfMinute(&var) };
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::from_i16(res as i16));
                }
                self.regs.eax = dst_ptr;
            }
            552 => {
                // rtcGetDayOfWeek(dst: *mut Variant, var: *const Variant, fdow: i32) (pops 12 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let var_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let fdow_val = self.mem.read_i32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let var = self.read_variant_for_host(var_ptr);
                let fdow = crate::types::Variant::from_i32(if fdow_val == 0 { 1 } else { fdow_val });
                if var.vt == crate::types::VT_NULL {
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::null());
                } else {
                    let res = unsafe { crate::rtc::extended::rtcGetDayOfWeek(&var, &fdow) };
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::from_i16(res));
                }
                self.regs.eax = dst_ptr;
            }
            553 => {
                // rtcGetYear(dst: *mut Variant, src: *const Variant) -> dst (stdcall, pops 8 bytes)
                // Returns a Variant (VT_I2) written into `dst`.
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let var_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let mut var = self.mem.read_variant_deref(var_ptr);
                var.vt &= 0x0FFF;
                if var.vt == crate::types::VT_BSTR {
                    let b = u32::from_le_bytes(var.data[..4].try_into().unwrap());
                    let txt = self.mem.read_bstr_string(b);
                    var = crate::rtc::extended::parse_ole_date_str(&txt)
                        .map(crate::types::Variant::date)
                        .unwrap_or_else(crate::types::Variant::empty);
                }
                if var.vt == crate::types::VT_NULL {
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::null());
                } else {
                    let res = unsafe { crate::rtc::extended::rtcGetYear(&var) };
                    self.mem.write_variant(dst_ptr, &crate::types::Variant::from_i16(res as i16));
                }
                self.regs.eax = dst_ptr;
            }
            556 => {
                // rtcIsArray (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let is_arr = (var.vt & crate::types::variant::VT_ARRAY) != 0;
                self.regs.eax = (if is_arr { -1i16 } else { 0i16 } as u16) as u32;
            }
            557 => {
                // rtcIsDate (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let base_vt = var.vt & 0x0FFF;
                let is_date = if base_vt == crate::types::VT_DATE {
                    true
                } else if base_vt == crate::types::VT_BSTR {
                    let bstr_ptr = u32::from_le_bytes(var.data[..4].try_into().unwrap());
                    let s = self.mem.read_bstr_string(bstr_ptr);
                    crate::rtc::extended::parse_ole_date_str(&s).is_some()
                } else {
                    false
                };
                self.regs.eax = (if is_date { -1i16 } else { 0i16 } as u16) as u32;
            }
            558 => {
                // rtcIsEmpty (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let is_empty = (var.vt & 0x0FFF) == crate::types::variant::VT_EMPTY;
                self.regs.eax = (if is_empty { -1i16 } else { 0i16 } as u16) as u32;
            }
            560 => {
                // rtcIsNull (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let is_null = (var.vt & 0x0FFF) == crate::types::variant::VT_NULL;
                self.regs.eax = (if is_null { -1i16 } else { 0i16 } as u16) as u32;
            }
            561 => {
                // rtcIsNumeric (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let base_vt = var.vt & 0x0FFF;
                let is_num = if base_vt == crate::types::VT_DATE || base_vt == crate::types::VT_BOOL {
                    false
                } else {
                    self.variant_to_f64(&var).is_some()
                };
                self.regs.eax = (if is_num { -1i16 } else { 0i16 } as u16) as u32;
            }
            562 => {
                // rtcIsObject (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let base_vt = var.vt & 0x0FFF;
                let is_obj = base_vt == crate::types::VT_DISPATCH || base_vt == crate::types::VT_UNKNOWN;
                self.regs.eax = (if is_obj { -1i16 } else { 0i16 } as u16) as u32;
            }
            661 => {
                // rtcDateAdd(dst, interval, number, date) (pops 20 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let int_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let number = f64::from_bits(self.mem.read_u64(self.regs.esp.wrapping_add(8)));
                let date_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(16));
                self.regs.esp = self.regs.esp.wrapping_add(20);
                let int_str = self.mem.read_bstr_string(int_ptr);
                let date_var = self.read_variant_for_host(date_ptr);
                let int_bstr = crate::types::bstr::sys_alloc_string(&int_str);
                let mut out_var = crate::types::Variant::empty();
                unsafe {
                    crate::rtc::extended::rtcDateAdd(&mut out_var, int_bstr, number, &date_var);
                    crate::types::bstr::SysFreeString(int_bstr);
                }
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            662 => {
                // rtcDateDiff(dst, interval, d1, d2, fdow, fwoy) (pops 24 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let int_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let d1_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let d2_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                let fdow_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(16));
                let fwoy_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(20));
                self.regs.esp = self.regs.esp.wrapping_add(24);
                let int_str = self.mem.read_bstr_string(int_ptr);
                let d1 = self.read_variant_for_host(d1_ptr);
                let d2 = self.read_variant_for_host(d2_ptr);
                let fdow = if fdow_ptr != 0 { self.read_variant_for_host(fdow_ptr) } else { crate::types::Variant::empty() };
                let fwoy = if fwoy_ptr != 0 { self.read_variant_for_host(fwoy_ptr) } else { crate::types::Variant::empty() };
                let int_bstr = crate::types::bstr::sys_alloc_string(&int_str);
                let mut out_var = crate::types::Variant::empty();
                unsafe {
                    crate::rtc::extended::rtcDateDiff(&mut out_var, int_bstr, &d1, &d2, &fdow, &fwoy);
                    crate::types::bstr::SysFreeString(int_bstr);
                }
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            663 => {
                // rtcDatePart(dst, interval, date, fdow, fwoy) (pops 20 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let int_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let date_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let fdow_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                let fwoy_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(16));
                self.regs.esp = self.regs.esp.wrapping_add(20);
                let int_str = self.mem.read_bstr_string(int_ptr);
                let date_var = self.read_variant_for_host(date_ptr);
                let fdow = if fdow_ptr != 0 { self.read_variant_for_host(fdow_ptr) } else { crate::types::Variant::empty() };
                let fwoy = if fwoy_ptr != 0 { self.read_variant_for_host(fwoy_ptr) } else { crate::types::Variant::empty() };
                let int_bstr = crate::types::bstr::sys_alloc_string(&int_str);
                let mut out_var = crate::types::Variant::empty();
                unsafe {
                    crate::rtc::extended::rtcDatePart(&mut out_var, int_bstr, &date_var, &fdow, &fwoy);
                    crate::types::bstr::SysFreeString(int_bstr);
                }
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            664 => {
                // rtcPartition(dst, num, start, stop, int) (pops 20 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let num_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let start_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let stop_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                let int_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(16));
                self.regs.esp = self.regs.esp.wrapping_add(20);
                let num = self.mem.read_variant_deref(num_ptr);
                let start = self.mem.read_variant_deref(start_ptr);
                let stop = self.mem.read_variant_deref(stop_ptr);
                let interval = self.mem.read_variant_deref(int_ptr);
                let mut out_var = crate::types::Variant::empty();
                unsafe { crate::rtc::extended::rtcPartition(&mut out_var, &num, &start, &stop, &interval) };
                if let Some(b) = out_var.as_bstr() {
                    let s = unsafe { crate::types::bstr::bstr_to_string(b) };
                    unsafe { crate::types::bstr::SysFreeString(b) };
                    let new_bstr = self.mem.alloc_bstr_from_str(&s);
                    let mut final_var = crate::types::Variant::empty();
                    final_var.vt = crate::types::variant::VT_BSTR;
                    final_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                    self.mem.write_variant(dst_ptr, &final_var);
                } else {
                    self.mem.write_variant(dst_ptr, &out_var);
                }
                self.regs.eax = dst_ptr;
            }
            437 => {
                // __vbaStrVarMove(v: *mut Variant) -> Bstr (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let bstr_ptr = if var.vt == crate::types::variant::VT_BSTR {
                    let ptr = u32::from_le_bytes(var.data[..4].try_into().unwrap());
                    self.mem.write_variant(var_ptr, &crate::types::Variant::empty());
                    ptr
                } else {
                    let s = self.variant_to_string(&var);
                    self.mem.write_variant(var_ptr, &crate::types::Variant::empty());
                    self.mem.alloc_bstr_from_str(&s)
                };
                self.regs.eax = bstr_ptr;
            }
            438 => {
                // __vbaStrVarVal(var: *const Variant) -> Bstr (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let s = self.variant_to_string(&var);
                let bstr_ptr = self.mem.alloc_bstr_from_str(&s);
                self.regs.eax = bstr_ptr;
            }
            429 => {
                // __vbaStrVarCopy(dst: *mut Bstr, src: *const Variant) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let var = self.mem.read_variant_deref(src_ptr);
                let s = self.variant_to_string(&var);
                let new_bstr = self.mem.alloc_bstr_from_str(&s);
                if dst_ptr != 0 {
                    self.mem.write_u32(dst_ptr, new_bstr);
                }
                self.regs.eax = 0;
            }
            668 => {
                // rtcSwitch(dst, pp_sa) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let pp_sa = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let p_sa = if pp_sa != 0 { self.mem.read_u32(pp_sa) } else { 0 };
                let mut matched = false;
                if p_sa != 0 {
                    let pv_data = self.mem.read_u32(p_sa.wrapping_add(12));
                    let count = self.mem.read_u32(p_sa.wrapping_add(16)) as usize;
                    let mut i = 0;
                    while i + 1 < count {
                        let cond = self.mem.read_variant_deref(pv_data.wrapping_add((i as u32) * 16));
                        let val = self.mem.read_variant_deref(pv_data.wrapping_add(((i + 1) as u32) * 16));
                        if cond.as_bool().unwrap_or(false) {
                            self.mem.write_variant(dst_ptr, &val);
                            matched = true;
                            break;
                        }
                        i += 2;
                    }
                }
                if !matched {
                    let null_var = crate::types::Variant::null();
                    self.mem.write_variant(dst_ptr, &null_var);
                }
                self.regs.eax = dst_ptr;
            }
            111 => {
                // __vbaStrErrVarCopy(p_var: *const Variant) -> Bstr (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let s = self.variant_to_string(&var);
                let bstr_ptr = self.mem.alloc_bstr_from_str(&s);
                self.regs.eax = bstr_ptr;
            }
            209 => {
                // __vbaDateVar(p_var: *const Variant) -> f64 (in ST0) (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let vt = var.vt & !crate::types::variant::VT_BYREF;
                let serial = match vt {
                    crate::types::VT_DATE | crate::types::VT_R8 => f64::from_le_bytes(var.data[0..8].try_into().unwrap()),
                    crate::types::VT_R4 => f32::from_le_bytes(var.data[0..4].try_into().unwrap()) as f64,
                    crate::types::VT_I2 => i16::from_le_bytes([var.data[0], var.data[1]]) as f64,
                    crate::types::VT_I4 => i32::from_le_bytes(var.data[0..4].try_into().unwrap()) as f64,
                    crate::types::VT_BSTR => {
                        let bstr_ptr = u32::from_le_bytes(var.data[0..4].try_into().unwrap());
                        let s = self.mem.read_bstr_string(bstr_ptr);
                        crate::rtc::extended::parse_ole_date_str(&s).unwrap_or(0.0)
                    }
                    _ => 0.0,
                };
                self.fpu.push(serial);
            }

            423 => {
                // __vbaStrR8(val: f64) -> Bstr (pops 8 bytes)
                let val = f64::from_bits(self.mem.read_u64(self.regs.esp));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let s = format!("{}", val);
                let res = self.mem.alloc_bstr_from_str(&s);
                self.regs.eax = res;
            }
            258 => {
                // __vbaHresultCheckObj(hr, obj, vtable_offset, info) (pops 16 bytes)
                let hr = self.mem.read_i32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(16);
                self.regs.eax = hr as u32;
            }
            340 | 341 => {
                // __vbaNew2 (340: target, p_desc) / __vbaNew (341: p_desc)
                let (target_ptr, p_desc) = if ordinal == 340 {
                    let d = self.mem.read_u32(self.regs.esp);
                    let t = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                    self.regs.esp = self.regs.esp.wrapping_add(8);
                    (t, d)
                } else {
                    let d = self.mem.read_u32(self.regs.esp);
                    self.regs.esp = self.regs.esp.wrapping_add(4);
                    (0, d)
                };

                let obj_base = self.mem.alloc_bytes(512);
                let iface_ptr = obj_base;
                let vtbl = self.mem.alloc_bytes(256);
                self.mem.write_u32(vtbl.wrapping_add(0x00), 0xDEAD0000 | 400); // QueryInterface
                self.mem.write_u32(vtbl.wrapping_add(0x04), 0xDEAD0000 | 401); // AddRef
                self.mem.write_u32(vtbl.wrapping_add(0x08), 0xDEAD0000 | 402); // Release
                self.mem.write_u32(vtbl.wrapping_add(0x0C), 0xDEAD0000 | 433); // GetTypeInfoCount
                self.mem.write_u32(vtbl.wrapping_add(0x10), 0xDEAD0000 | 434); // GetTypeInfo
                self.mem.write_u32(vtbl.wrapping_add(0x14), 0xDEAD0000 | 403); // GetIDsOfNames
                self.mem.write_u32(vtbl.wrapping_add(0x18), 0xDEAD0000 | 404); // Invoke

                if p_desc != 0 {
                    let header_flags = self.mem.read_u32(p_desc);
                    if header_flags == 0 {
                        // External COM class descriptor (e.g. VBA.Collection)
                        self.mem.write_u32(vtbl.wrapping_add(0x1C), 0xDEAD0000 | 0x8201); // Item
                        self.mem.write_u32(vtbl.wrapping_add(0x20), 0xDEAD0000 | 0x8202); // Add
                        self.mem.write_u32(vtbl.wrapping_add(0x24), 0xDEAD0000 | 0x8203); // Count
                        self.mem.write_u32(vtbl.wrapping_add(0x28), 0xDEAD0000 | 0x8204); // Remove
                        self.mem.write_u32(vtbl.wrapping_add(0x2C), 0xDEAD0000 | 0x8205); // _NewEnum
                        self.collections.insert(iface_ptr, Vec::new());
                    } else {
                        let method_count = self.mem.read_u32(p_desc.wrapping_add(0x60));
                        let method_table = self.mem.read_u32(p_desc.wrapping_add(0x68));
                        if method_count > 0 && method_count < 256 && method_table != 0 {
                            for i in 0..method_count {
                                let jump_ptr = self.mem.read_u32(method_table.wrapping_add(i * 4));
                                self.mem.write_u32(vtbl.wrapping_add(0x1C + i * 4), jump_ptr);
                            }
                        }
                    }
                }

                self.mem.write_u32(obj_base, vtbl);
                if target_ptr != 0 {
                    self.mem.write_u32(target_ptr, iface_ptr);
                }
                self.regs.eax = iface_ptr;
            }
            400 | 410 | 430 => {
                // QueryInterface / EVENT_SINK_QueryInterface / BASIC_CLASS_QueryInterface / Zombie_QueryInterface (pops 12 bytes)
                let this = self.mem.read_u32(self.regs.esp);
                let ppv = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                if ppv != 0 {
                    self.mem.write_u32(ppv, this);
                }
                self.regs.eax = 0; // S_OK
            }
            401 | 411 | 431 | 440 => {
                // AddRef (pops 4 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = 2;
            }
            402 | 412 | 432 | 441 => {
                // Release (pops 4 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = 1;
            }
            420 | 433 => {
                // GetTypeInfoCount / Zombie_GetTypeInfoCount(this, pctinfo) (pops 8 bytes)
                let pctinfo = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if pctinfo != 0 {
                    self.mem.write_u32(pctinfo, 0);
                }
                self.regs.eax = 0;
            }
            421 | 434 => {
                // GetTypeInfo / Zombie_GetTypeInfo(this, iTInfo, lcid, ppTInfo) (pops 16 bytes)
                let pp_tinfo = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                self.regs.esp = self.regs.esp.wrapping_add(16);
                if pp_tinfo != 0 {
                    self.mem.write_u32(pp_tinfo, 0);
                }
                self.regs.eax = 0;
            }
            403 | 413 | 435 => {
                // GetIDsOfNames(this, riid, rgszNames, cNames, lcid, rgDispId) (pops 24 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(24);
                self.regs.eax = 0;
            }
            404 | 414 | 436 => {
                // Invoke(this, dispIdMember, riid, lcid, wFlags, pDispParams, pVarResult, pExcepInfo, puArgErr) (pops 36 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(36);
                self.regs.eax = 0;
            }
            300..=321 => {
                // Memory / Property / Event Sink accessors:
                // 300: GetMem2, 301: GetMem4, 302: GetMem8, 303: GetMemStr, 304: GetMemVar, 305: GetMemObj
                // 306: PutMem2, 307: PutMem4, 308: PutMem8, 309: PutMemStr, 310: PutMemVar, 311: PutMemObj
                // 312: SetMemVar, 313: SetMemObj, 314: GetMemNewObj, 315: PutMemNewObj, 316: SetMemNewObj
                // 317: GetMem1, 318: PutMem1
                // 319: GetMemEvent, 320: PutMemEvent, 321: SetMemEvent
                match ordinal {
                    317 => {
                        // GetMem1(src_ptr, dst_ptr)
                        let src = self.mem.read_u32(self.regs.esp);
                        let dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(8);
                        if src != 0 && dst != 0 {
                            let val = self.mem.read_u8(src);
                            self.mem.write_u8(dst, val);
                        }
                        self.regs.eax = 0;
                    }
                    318 => {
                        // PutMem1(dst_ptr, val)
                        let dst = self.mem.read_u32(self.regs.esp);
                        let val = self.mem.read_u8(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(8);
                        if dst != 0 {
                            self.mem.write_u8(dst, val);
                        }
                        self.regs.eax = 0;
                    }
                    300 => {
                        // GetMem2(src_ptr, dst_ptr)
                        let src = self.mem.read_u32(self.regs.esp);
                        let dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(8);
                        if src != 0 && dst != 0 {
                            let val = self.mem.read_u16(src);
                            self.mem.write_u16(dst, val);
                        }
                        self.regs.eax = 0;
                    }
                    306 => {
                        // PutMem2(dst_ptr, val)
                        let dst = self.mem.read_u32(self.regs.esp);
                        let val = self.mem.read_u16(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(8);
                        if dst != 0 {
                            self.mem.write_u16(dst, val);
                        }
                        self.regs.eax = 0;
                    }
                    301 | 305 => {
                        // GetMem4 / GetMemObj(src_ptr, dst_ptr)
                        let src = self.mem.read_u32(self.regs.esp);
                        let dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(8);
                        if src != 0 && dst != 0 {
                            let val = self.mem.read_u32(src);
                            self.mem.write_u32(dst, val);
                        }
                        self.regs.eax = 0;
                    }
                    307 | 311 | 313 => {
                        // PutMem4 / PutMemObj / SetMemObj(dst_ptr, val)
                        let dst = self.mem.read_u32(self.regs.esp);
                        let val = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(8);
                        if dst != 0 {
                            self.mem.write_u32(dst, val);
                        }
                        self.regs.eax = 0;
                    }
                    302 => {
                        // GetMem8(src_ptr, dst_ptr)
                        let src = self.mem.read_u32(self.regs.esp);
                        let dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(8);
                        if src != 0 && dst != 0 {
                            let val = self.mem.read_u64(src);
                            self.mem.write_u64(dst, val);
                        }
                        self.regs.eax = 0;
                    }
                    308 => {
                        // PutMem8(dst_ptr, val_lo, val_hi) (pops 12 bytes)
                        let dst = self.mem.read_u32(self.regs.esp);
                        let val = self.mem.read_u64(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(12);
                        if dst != 0 {
                            self.mem.write_u64(dst, val);
                        }
                        self.regs.eax = 0;
                    }
                    303 => {
                        // GetMemStr(src_ptr, dst_ptr)
                        let src = self.mem.read_u32(self.regs.esp);
                        let dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(8);
                        if dst != 0 {
                            let bstr = if src != 0 { self.mem.read_u32(src) } else { 0 };
                            let cloned = if bstr != 0 {
                                let chars = self.mem.read_bstr(bstr);
                                self.mem.alloc_bstr(&chars)
                            } else {
                                0
                            };
                            self.mem.write_u32(dst, cloned);
                        }
                        self.regs.eax = 0;
                    }
                    309 => {
                        // PutMemStr(dst_ptr, new_bstr)
                        let dst = self.mem.read_u32(self.regs.esp);
                        let new_bstr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(8);
                        if dst != 0 {
                            let cloned = if new_bstr != 0 {
                                let chars = self.mem.read_bstr(new_bstr);
                                self.mem.alloc_bstr(&chars)
                            } else {
                                0
                            };
                            self.mem.write_u32(dst, cloned);
                        }
                        self.regs.eax = 0;
                    }
                    304 => {
                        // GetMemVar(src_ptr, dst_ptr)
                        let src = self.mem.read_u32(self.regs.esp);
                        let dst = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(8);
                        if src != 0 && dst != 0 {
                            let var = self.mem.read_variant_deref(src);
                            self.mem.write_variant(dst, &var);
                        }
                        self.regs.eax = 0;
                    }
                    310 | 312 => {
                        // PutMemVar / SetMemVar(dst_ptr, src_var_ on stack or ptr)
                        let dst = self.mem.read_u32(self.regs.esp);
                        let src = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                        self.regs.esp = self.regs.esp.wrapping_add(8);
                        if dst != 0 && src != 0 {
                            let var = self.mem.read_variant_deref(src);
                            self.mem.write_variant(dst, &var);
                        }
                        self.regs.eax = 0;
                    }
                    319 => {
                        // GetMemEvent(class_desc, event_idx, sink_slot_ptr, p_obj_out) (pops 16 bytes)
                        let sink_slot_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                        let p_obj_out = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                        self.regs.esp = self.regs.esp.wrapping_add(16);
                        if p_obj_out != 0 {
                            let obj = if sink_slot_ptr != 0 { self.mem.read_u32(sink_slot_ptr) } else { 0 };
                            self.mem.write_u32(p_obj_out, obj);
                        }
                        self.regs.eax = 0;
                    }
                    320 | 321 => {
                        // PutMemEvent / SetMemEvent(class_desc, event_idx, sink_slot_ptr, source_obj) (pops 16 bytes)
                        let class_desc = self.mem.read_u32(self.regs.esp);
                        let _event_idx = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                        let sink_slot_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                        let source_obj = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                        self.regs.esp = self.regs.esp.wrapping_add(16);

                        if sink_slot_ptr != 0 {
                            let sink_this = sink_slot_ptr.wrapping_add(4);
                            let old_source = self.mem.read_u32(sink_slot_ptr);
                            if old_source != 0 {
                                if let Some(sinks) = self.event_sinks.get_mut(&old_source) {
                                    sinks.retain(|&s| s != sink_this);
                                }
                            }
                            self.mem.write_u32(sink_slot_ptr, source_obj);

                            // Locate the event sink vtable in class_desc
                            let mut sink_vtbl = 0;
                            if class_desc != 0 {
                                for off in (0x80..0x300u32).step_by(4) {
                                    let addr = class_desc.wrapping_add(off);
                                    if self.mem.read_u32(addr) == class_desc {
                                        let candidate_vtbl = addr.wrapping_add(4);
                                        if self.mem.read_u32(candidate_vtbl.wrapping_add(0x0C)) >= 0x00401000
                                            && self.mem.read_u32(candidate_vtbl.wrapping_add(0x1C)) >= 0x00401000
                                        {
                                            sink_vtbl = candidate_vtbl;
                                            break;
                                        }
                                    }
                                }
                            }
                            if sink_vtbl != 0 {
                                self.mem.write_u32(sink_this, sink_vtbl);
                            }
                            if source_obj != 0 {
                                let sinks = self.event_sinks.entry(source_obj).or_default();
                                if !sinks.contains(&sink_this) {
                                    sinks.push(sink_this);
                                }
                            }
                        }
                        self.regs.eax = 0;
                    }
                    _ => {
                        self.regs.esp = self.regs.esp.wrapping_add(16);
                        self.regs.eax = 0;
                    }
                }
            }
            379 => {
                // __vbaRaiseEvent(source_obj, event_id, num_args, ...variants) - cdecl (caller cleans stack)
                let source_obj = self.mem.read_u32(self.regs.esp);
                let event_id = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let num_args = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                if let Some(sinks) = self.event_sinks.get(&source_obj).cloned() {
                    if let Some(&sink_this) = sinks.first() {
                        let sink_vtbl = self.mem.read_u32(sink_this);
                        if sink_vtbl != 0 && event_id > 0 {
                            let handler_addr = self.mem.read_u32(sink_vtbl.wrapping_add(0x18 + event_id * 4));
                            if handler_addr != 0 {
                                let orig_esp = self.regs.esp;
                                let mut payload_args = Vec::with_capacity(num_args as usize);
                                for i in 0..num_args {
                                    let var_addr = orig_esp.wrapping_add(12 + i * 16);
                                    let var = self.mem.read_variant_deref(var_addr);
                                    let d = u32::from_le_bytes(var.data[0..4].try_into().unwrap());
                                    payload_args.push(d);
                                }
                                for d in payload_args {
                                    self.push32(d);
                                }
                                self.push32(sink_this);
                                self.push32(ret_addr);
                                self.regs.eip = handler_addr;
                                self.error_redirected = true;
                            }
                        }
                    }
                }
                self.regs.eax = 0;
            }
            0x8201 => {
                // VBA.Collection::Item(this, index_var_ptr, out_var_ptr) -> HRESULT (pops 12 bytes)
                let this = self.mem.read_u32(self.regs.esp);
                let index_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let out_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let idx_var = self.mem.read_variant_deref(index_ptr);
                let found = if let Some(items) = self.collections.get(&this) {
                    if (idx_var.vt & 0x0FFF) == crate::types::VT_BSTR {
                        let key = self.variant_to_string(&idx_var);
                        items
                            .iter()
                            .find(|(k, _)| k.as_deref().map(|s| s.eq_ignore_ascii_case(&key)).unwrap_or(false))
                            .map(|(_, v)| crate::types::Variant::from_bytes(*v))
                    } else {
                        let idx = idx_var.as_i32().unwrap_or(0);
                        if idx >= 1 && (idx as usize) <= items.len() {
                            Some(crate::types::Variant::from_bytes(items[(idx - 1) as usize].1))
                        } else {
                            None
                        }
                    }
                } else {
                    None
                };
                if let Some(mut v) = found {
                    if (v.vt & 0x0FFF) == crate::types::VT_BSTR {
                        let bstr_ptr = u32::from_le_bytes(v.data[0..4].try_into().unwrap());
                        let chars = self.mem.read_bstr(bstr_ptr);
                        let dup = self.mem.alloc_bstr(&chars);
                        v.data[0..4].copy_from_slice(&dup.to_le_bytes());
                    }
                    if out_ptr != 0 {
                        self.mem.write_variant(out_ptr, &v);
                    }
                    self.regs.eax = 0;
                } else {
                    self.handle_error(5)?;
                }
            }
            0x8202 => {
                // VBA.Collection::Add(this, item_ptr, key_ptr, before_ptr, after_ptr) -> HRESULT (pops 20 bytes)
                let this = self.mem.read_u32(self.regs.esp);
                let item_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let key_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(20);
                let mut item_var = self.mem.read_variant_deref(item_ptr);
                if (item_var.vt & 0x0FFF) == crate::types::VT_BSTR {
                    let bstr_ptr = u32::from_le_bytes(item_var.data[0..4].try_into().unwrap());
                    let chars = self.mem.read_bstr(bstr_ptr);
                    let dup = self.mem.alloc_bstr(&chars);
                    item_var.data[0..4].copy_from_slice(&dup.to_le_bytes());
                }
                let key = if key_ptr != 0 {
                    let kv = self.mem.read_variant_deref(key_ptr);
                    if (kv.vt & 0x0FFF) == crate::types::VT_BSTR {
                        Some(self.variant_to_string(&kv))
                    } else {
                        None
                    }
                } else {
                    None
                };
                self.collections
                    .entry(this)
                    .or_default()
                    .push((key, item_var.to_bytes()));
                self.regs.eax = 0;
            }
            0x8203 => {
                // VBA.Collection::Count(this, p_count_out) -> HRESULT (pops 8 bytes)
                let this = self.mem.read_u32(self.regs.esp);
                let p_count = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let count = self.collections.get(&this).map(|v| v.len() as u32).unwrap_or(0);
                if p_count != 0 {
                    self.mem.write_u32(p_count, count);
                }
                self.regs.eax = 0;
            }
            0x8204 => {
                // VBA.Collection::Remove(this, index_ptr) -> HRESULT (pops 8 bytes)
                let this = self.mem.read_u32(self.regs.esp);
                let index_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let idx_var = self.mem.read_variant_deref(index_ptr);
                let is_str = (idx_var.vt & 0x0FFF) == crate::types::VT_BSTR;
                let key_str = if is_str { self.variant_to_string(&idx_var) } else { String::new() };
                let idx_num = if is_str { 0 } else { idx_var.as_i32().unwrap_or(0) };
                if let Some(items) = self.collections.get_mut(&this) {
                    if is_str {
                        if let Some(pos) = items
                            .iter()
                            .position(|(k, _)| k.as_deref().map(|s| s.eq_ignore_ascii_case(&key_str)).unwrap_or(false))
                        {
                            items.remove(pos);
                        }
                    } else if idx_num >= 1 && (idx_num as usize) <= items.len() {
                        items.remove((idx_num - 1) as usize);
                    }
                }
                self.regs.eax = 0;
            }
            0x8205 => {
                // VBA.Collection::_NewEnum(this, ppunk_out) -> HRESULT (pops 8 bytes)
                let this = self.mem.read_u32(self.regs.esp);
                let ppunk = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if ppunk != 0 {
                    self.mem.write_u32(ppunk, this);
                }
                self.regs.eax = 0;
            }
            240 => {
                // __vbaForEachCollObj(iid_ptr, p_iter_state, p_loop_obj, col_obj) -> i32 (pops 16 bytes)
                let _iid_ptr = self.mem.read_u32(self.regs.esp);
                let p_iter = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let p_loop_obj = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let col_obj = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                self.regs.esp = self.regs.esp.wrapping_add(16);
                let first_obj = self
                    .collections
                    .get(&col_obj)
                    .and_then(|items| items.first())
                    .map(|(_, v)| u32::from_le_bytes(v[8..12].try_into().unwrap()));
                if let Some(obj_ptr) = first_obj {
                    if p_loop_obj != 0 {
                        self.mem.write_u32(p_loop_obj, obj_ptr);
                    }
                    if p_iter != 0 {
                        self.mem.write_u32(p_iter, 1);
                        self.coll_iterators.insert(p_iter, (col_obj, 1));
                    }
                    self.regs.eax = 1;
                } else {
                    if p_loop_obj != 0 {
                        self.mem.write_u32(p_loop_obj, 0);
                    }
                    if p_iter != 0 {
                        self.mem.write_u32(p_iter, 0);
                    }
                    self.regs.eax = 0;
                }
            }
            344 => {
                // __vbaNextEachCollObj(iid_ptr, p_iter_state, p_loop_obj) -> i32 (pops 12 bytes)
                let _iid_ptr = self.mem.read_u32(self.regs.esp);
                let p_iter = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let p_loop_obj = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let state = self.coll_iterators.get(&p_iter).copied();
                let next_obj = state.and_then(|(col_obj, idx)| {
                    self.collections
                        .get(&col_obj)
                        .and_then(|items| items.get(idx))
                        .map(|(_, v)| (col_obj, idx + 1, u32::from_le_bytes(v[8..12].try_into().unwrap())))
                });
                if let Some((col_obj, next_idx, obj_ptr)) = next_obj {
                    if p_loop_obj != 0 {
                        self.mem.write_u32(p_loop_obj, obj_ptr);
                    }
                    self.coll_iterators.insert(p_iter, (col_obj, next_idx));
                    self.regs.eax = 1;
                } else {
                    self.coll_iterators.remove(&p_iter);
                    if p_loop_obj != 0 {
                        self.mem.write_u32(p_loop_obj, 0);
                    }
                    if p_iter != 0 {
                        self.mem.write_u32(p_iter, 0);
                    }
                    self.regs.eax = 0;
                }
            }
            241 => {
                // __vbaForEachCollVar(p_iter_state, p_loop_var, col_obj) -> i32 (pops 12 bytes)
                let p_iter = self.mem.read_u32(self.regs.esp);
                let p_loop_var = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let col_obj = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let first_var = self
                    .collections
                    .get(&col_obj)
                    .and_then(|items| items.first())
                    .map(|(_, v)| crate::types::Variant::from_bytes(*v));
                if let Some(mut v) = first_var {
                    if (v.vt & 0x0FFF) == crate::types::VT_BSTR {
                        let bstr_ptr = u32::from_le_bytes(v.data[0..4].try_into().unwrap());
                        let chars = self.mem.read_bstr(bstr_ptr);
                        let dup = self.mem.alloc_bstr(&chars);
                        v.data[0..4].copy_from_slice(&dup.to_le_bytes());
                    }
                    if p_loop_var != 0 {
                        self.mem.write_variant(p_loop_var, &v);
                    }
                    if p_iter != 0 {
                        self.mem.write_u32(p_iter, 1);
                        self.coll_iterators.insert(p_iter, (col_obj, 1));
                    }
                    self.regs.eax = 1;
                } else {
                    if p_loop_var != 0 {
                        self.mem.write_variant(p_loop_var, &crate::types::Variant::empty());
                    }
                    if p_iter != 0 {
                        self.mem.write_u32(p_iter, 0);
                    }
                    self.regs.eax = 0;
                }
            }
            345 => {
                // __vbaNextEachCollVar(p_iter_state, p_loop_var) -> i32 (pops 8 bytes)
                let p_iter = self.mem.read_u32(self.regs.esp);
                let p_loop_var = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let state = self.coll_iterators.get(&p_iter).copied();
                let next_var = state.and_then(|(col_obj, idx)| {
                    self.collections
                        .get(&col_obj)
                        .and_then(|items| items.get(idx))
                        .map(|(_, v)| (col_obj, idx + 1, crate::types::Variant::from_bytes(*v)))
                });
                if let Some((col_obj, next_idx, mut v)) = next_var {
                    if (v.vt & 0x0FFF) == crate::types::VT_BSTR {
                        let bstr_ptr = u32::from_le_bytes(v.data[0..4].try_into().unwrap());
                        let chars = self.mem.read_bstr(bstr_ptr);
                        let dup = self.mem.alloc_bstr(&chars);
                        v.data[0..4].copy_from_slice(&dup.to_le_bytes());
                    }
                    if p_loop_var != 0 {
                        self.mem.write_variant(p_loop_var, &v);
                    }
                    self.coll_iterators.insert(p_iter, (col_obj, next_idx));
                    self.regs.eax = 1;
                } else {
                    self.coll_iterators.remove(&p_iter);
                    if p_loop_var != 0 {
                        self.mem.write_variant(p_loop_var, &crate::types::Variant::empty());
                    }
                    if p_iter != 0 {
                        self.mem.write_u32(p_iter, 0);
                    }
                    self.regs.eax = 0;
                }
            }
            221 => {
                // __vbaExitEachColl(p_iter_state) (pops 4 bytes)
                let p_iter = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.coll_iterators.remove(&p_iter);
                if p_iter != 0 {
                    self.mem.write_u32(p_iter, 0);
                }
                self.regs.eax = 0;
            }
            223 => {
                // __vbaExitProc()
                self.regs.eax = 0;
            }
            352 => {
                // __vbaOnError(flag) (pops 4 bytes)
                let flag = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let curr_record = self.mem.read_u32(self.regs.fs_base);
                if curr_record != 0 && curr_record != 0xFFFFFFFF {
                    self.active_error_handlers.insert(curr_record, flag);
                }
                self.regs.eax = 0;
            }
            217 => {
                // __vbaErrorOverflow()
                return self.handle_error(6);
            }
            218 => {
                // __vbaExceptHandler()
                self.regs.eax = 0;
            }
            391 => {
                // __vbaResume(target: i32) (pops 4 bytes)
                let target = self.mem.read_i32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let curr_record = self.mem.read_u32(self.regs.fs_base);
                let table_ptr = if curr_record != 0 && curr_record != 0xFFFFFFFF {
                    self.mem.read_u32(curr_record.wrapping_add(12))
                } else {
                    self.mem.read_u32(self.regs.ebp.wrapping_sub(0x14))
                };
                if table_ptr != 0 {
                    let count = self.mem.read_u32(table_ptr.wrapping_add(0x18)) as usize;
                    let scopes_base = table_ptr.wrapping_add(0x1C);
                    let saved_esp = if curr_record != 0 && curr_record != 0xFFFFFFFF {
                        self.mem.read_u32(curr_record.wrapping_add(8))
                    } else {
                        0
                    };
                    if target == -2 {
                        // Resume: re-execute statement that caused the error
                        let scope = self.error_scope;
                        if scope > 0 && scope <= count {
                            if saved_esp != 0 {
                                self.regs.esp = saved_esp;
                            }
                            let resume_eip = self.mem.read_u32(scopes_base.wrapping_add((scope as u32 - 1) * 4));
                            self.regs.eip = resume_eip;
                            return Ok(());
                        }
                    } else if target == -1 {
                        // Resume Next: execute statement following the statement that caused the error
                        let scope = self.error_scope;
                        if scope < count {
                            if saved_esp != 0 {
                                self.regs.esp = saved_esp;
                            }
                            let resume_eip = self.mem.read_u32(scopes_base.wrapping_add(scope as u32 * 4));
                            self.regs.eip = resume_eip;
                            return Ok(());
                        }
                    } else if target == 0 {
                        // Resume <Label>: compiler emits `jmp <Label>` immediately at `ret_addr`
                        self.regs.eip = ret_addr;
                        return Ok(());
                    } else if target > 0 && (target as usize) < count {
                        if saved_esp != 0 {
                            self.regs.esp = saved_esp;
                        }
                        let resume_eip = self.mem.read_u32(scopes_base.wrapping_add(target as u32 * 4));
                        self.regs.eip = resume_eip;
                        return Ok(());
                    }
                }
                return Ok(());
            }
            685 => {
                // rtcErrObj() -> Err object in EAX
                let vtbl = self.mem.alloc_bytes(128);
                self.mem.write_u32(vtbl.wrapping_add(0x00), 0xDEAD0000 | 400); // QueryInterface
                self.mem.write_u32(vtbl.wrapping_add(0x04), 0xDEAD0000 | 401); // AddRef
                self.mem.write_u32(vtbl.wrapping_add(0x08), 0xDEAD0000 | 402); // Release
                self.mem.write_u32(vtbl.wrapping_add(0x1C), 0xDEAD0000 | 0x8101); // Err.Number get
                self.mem.write_u32(vtbl.wrapping_add(0x20), 0xDEAD0000 | 0x8104); // Err.Number put
                self.mem.write_u32(vtbl.wrapping_add(0x24), 0xDEAD0000 | 0x8105); // Err.Source get
                self.mem.write_u32(vtbl.wrapping_add(0x28), 0xDEAD0000 | 0x8106); // Err.Source put
                self.mem.write_u32(vtbl.wrapping_add(0x2C), 0xDEAD0000 | 0x8107); // Err.Description get
                self.mem.write_u32(vtbl.wrapping_add(0x30), 0xDEAD0000 | 0x8108); // Err.Description put
                self.mem.write_u32(vtbl.wrapping_add(0x34), 0xDEAD0000 | 0x8109); // Err.HelpFile get
                self.mem.write_u32(vtbl.wrapping_add(0x38), 0xDEAD0000 | 0x810A); // Err.HelpFile put
                self.mem.write_u32(vtbl.wrapping_add(0x3C), 0xDEAD0000 | 0x810B); // Err.HelpContext get
                self.mem.write_u32(vtbl.wrapping_add(0x40), 0xDEAD0000 | 0x810C); // Err.HelpContext put
                self.mem.write_u32(vtbl.wrapping_add(0x44), 0xDEAD0000 | 0x8102); // Err.Raise
                self.mem.write_u32(vtbl.wrapping_add(0x48), 0xDEAD0000 | 0x8103); // Err.Clear
                self.mem.write_u32(vtbl.wrapping_add(0x4C), 0xDEAD0000 | 0x810D); // Err.LastDllError get
                let obj = self.mem.alloc_bytes(64);
                self.mem.write_u32(obj, vtbl);
                self.regs.eax = obj;
            }
            0x8101 => {
                // Err.Number(this, p_num_out) (pops 8 bytes)
                let _this = self.mem.read_u32(self.regs.esp);
                let p_num = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if p_num != 0 {
                    self.mem.write_u32(p_num, self.last_error_number);
                }
                self.regs.eax = 0; // S_OK
            }
            0x8104 => {
                // Err.put_Number(this, num) (pops 8 bytes)
                let _this = self.mem.read_u32(self.regs.esp);
                let num = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.last_error_number = num;
                self.regs.eax = 0; // S_OK
            }
            0x8105 => {
                // Err.get_Source(this, p_bstr_out) (pops 8 bytes)
                let _this = self.mem.read_u32(self.regs.esp);
                let p_bstr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if p_bstr != 0 {
                    let s = self.last_error_source.clone();
                    let bstr = self.mem.alloc_bstr_from_str(&s);
                    self.mem.write_u32(p_bstr, bstr);
                }
                self.regs.eax = 0; // S_OK
            }
            0x8106 => {
                // Err.put_Source(this, bstr) (pops 8 bytes)
                let _this = self.mem.read_u32(self.regs.esp);
                let bstr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.last_error_source = self.mem.read_bstr_string(bstr);
                self.regs.eax = 0;
            }
            0x8107 | 0x8109 => {
                // Err.get_Description / get_HelpFile (this, p_bstr_out) (pops 8 bytes)
                let _this = self.mem.read_u32(self.regs.esp);
                let p_bstr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if p_bstr != 0 {
                    let s = self.last_error_description.clone();
                    let bstr = self.mem.alloc_bstr_from_str(&s);
                    self.mem.write_u32(p_bstr, bstr);
                }
                self.regs.eax = 0;
            }
            0x8108 | 0x810A => {
                // Err.put_Description / put_HelpFile (this, bstr) (pops 8 bytes)
                let _this = self.mem.read_u32(self.regs.esp);
                let bstr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.last_error_description = self.mem.read_bstr_string(bstr);
                self.regs.eax = 0;
            }
            0x810B | 0x810D => {
                // Err.get_HelpContext / get_LastDllError (this, p_out) (pops 8 bytes)
                let _this = self.mem.read_u32(self.regs.esp);
                let p_out = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                if p_out != 0 {
                    self.mem.write_u32(p_out, 0);
                }
                self.regs.eax = 0;
            }
            0x810C => {
                // Err.put_HelpContext (this, val) (pops 8 bytes)
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.regs.eax = 0;
            }
            0x8102 => {
                // Err.Raise(this, number, source, desc, helpfile, helpcontext) (pops 24 bytes)
                let _this = self.mem.read_u32(self.regs.esp);
                let number = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let desc_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                self.regs.esp = self.regs.esp.wrapping_add(24);
                if src_ptr != 0 {
                    let v = self.mem.read_variant_deref(src_ptr);
                    if (v.vt & 0x0FFF) != crate::types::VT_ERROR && (v.vt & 0x0FFF) != crate::types::VT_EMPTY {
                        self.last_error_source = self.variant_to_string(&v);
                    }
                }
                if desc_ptr != 0 {
                    let v = self.mem.read_variant_deref(desc_ptr);
                    if (v.vt & 0x0FFF) != crate::types::VT_ERROR && (v.vt & 0x0FFF) != crate::types::VT_EMPTY {
                        self.last_error_description = self.variant_to_string(&v);
                    }
                }
                return self.handle_error(number);
            }
            0x8103 => {
                // Err.Clear(this) (pops 4 bytes)
                let _this = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.last_error_number = 0;
                self.last_error_source.clear();
                self.last_error_description.clear();
                self.regs.eax = 0; // S_OK
            }
            516 => {
                // rtcAnsiValueBstr(bstr) -> i16 (Asc) (pops 4 bytes)
                let bstr_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let ch = if bstr_ptr != 0 { self.mem.read_u16(bstr_ptr) & 0xFF } else { 0 };
                self.regs.eax = (ch as i16 as u16) as u32;
            }
            518 => {
                // rtcLowerCaseVar(dst, src) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let var = self.mem.read_variant_deref(src_ptr);
                let s = self.variant_to_string(&var).to_lowercase();
                let new_bstr = self.mem.alloc_bstr_from_str(&s);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            520 => {
                // rtcTrimVar(dst, src) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let var = self.mem.read_variant_deref(src_ptr);
                let s = self.variant_to_string(&var);
                let trimmed = s.trim();
                let new_bstr = self.mem.alloc_bstr_from_str(trimmed);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            522 => {
                // rtcLeftTrimVar(dst, src) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let var = self.mem.read_variant_deref(src_ptr);
                let s = self.variant_to_string(&var);
                let trimmed = s.trim_start();
                let new_bstr = self.mem.alloc_bstr_from_str(trimmed);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            524 => {
                // rtcRightTrimVar(dst, src) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let var = self.mem.read_variant_deref(src_ptr);
                let s = self.variant_to_string(&var);
                let trimmed = s.trim_end();
                let new_bstr = self.mem.alloc_bstr_from_str(trimmed);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            526 => {
                // rtcSpaceVar(dst, n) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let n = self.mem.read_i32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let s = " ".repeat(n.max(0) as usize);
                let new_bstr = self.mem.alloc_bstr_from_str(&s);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            528 => {
                // rtcUpperCaseVar(dst, src) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let var = self.mem.read_variant_deref(src_ptr);
                let s = self.variant_to_string(&var).to_uppercase();
                let new_bstr = self.mem.alloc_bstr_from_str(&s);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            559 => {
                // rtcIsError(var) -> i16 (pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let var = self.mem.read_variant_deref(var_ptr);
                let res: i16 = if (var.vt & !crate::types::variant::VT_BYREF) == crate::types::variant::VT_ERROR {
                    -1
                } else {
                    0
                };
                self.regs.eax = (res as u16) as u32;
            }
            573 => {
                // rtcHexVarFromVar(dst, var) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let var_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let var = self.mem.read_variant_deref(var_ptr);
                let val = var.as_i32().unwrap_or(0);
                let s = format!("{:X}", val);
                let new_bstr = self.mem.alloc_bstr_from_str(&s);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            575 => {
                // rtcOctVarFromVar(dst, var) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let var_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let var = self.mem.read_variant_deref(var_ptr);
                let val = var.as_i32().unwrap_or(0);
                let s = format!("{:o}", val);
                let new_bstr = self.mem.alloc_bstr_from_str(&s);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            589 => {
                // rtcQBColor(color_index) (pops 4 bytes)
                let color_index = self.mem.read_i32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let res = unsafe { crate::rtc::complete::rtcQBColor(color_index) };
                self.regs.eax = res as u32;
            }
            607 => {
                // rtcStringVar(dst, n, char_var) (pops 12 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let n = self.mem.read_i32(self.regs.esp.wrapping_add(4));
                let char_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let var = self.mem.read_variant_deref(char_ptr);
                let ch_str = self.variant_to_string(&var);
                let ch = ch_str.chars().next().unwrap_or(' ');
                let s: String = std::iter::repeat_n(ch, n.max(0) as usize).collect();
                let new_bstr = self.mem.alloc_bstr_from_str(&s);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            608 => {
                // rtcVarBstrFromAnsi(dst, charcode) (Chr) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let ch = self.mem.read_u16(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let new_bstr = self.mem.alloc_bstr(&[ch & 0xFF]);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            610 => {
                // rtcGetDateVar(dst) (pops 4 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let now = std::time::SystemTime::now();
                let epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                let days_from_1970 = (epoch.as_secs() / 86400) as f64;
                let ole_date = days_from_1970 + 25569.0;
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_DATE;
                out_var.data[0..8].copy_from_slice(&ole_date.to_ne_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            612 => {
                // rtcGetTimeVar(dst) (pops 4 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let now = std::time::SystemTime::now();
                let epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                let secs_of_day = (epoch.as_secs() % 86400) as f64;
                let ole_time = secs_of_day / 86400.0;
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_DATE;
                out_var.data[0..8].copy_from_slice(&ole_time.to_ne_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            617 => {
                // rtcLeftCharVar(dst, str_var, len) (pops 12 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let str_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let len = self.mem.read_i32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let var = self.mem.read_variant_deref(str_ptr);
                let s = self.variant_to_string(&var);
                let u16_chars: Vec<u16> = s.encode_utf16().collect();
                let count = (len.max(0) as usize).min(u16_chars.len());
                let sub = String::from_utf16_lossy(&u16_chars[..count]);
                let new_bstr = self.mem.alloc_bstr_from_str(&sub);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            619 => {
                // rtcRightCharVar(dst, str_var, len) (pops 12 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let str_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let len = self.mem.read_i32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let var = self.mem.read_variant_deref(str_ptr);
                let s = self.variant_to_string(&var);
                let u16_chars: Vec<u16> = s.encode_utf16().collect();
                let count = (len.max(0) as usize).min(u16_chars.len());
                let start_idx = u16_chars.len().saturating_sub(count);
                let sub = String::from_utf16_lossy(&u16_chars[start_idx..]);
                let new_bstr = self.mem.alloc_bstr_from_str(&sub);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            631 => {
                // rtcMidCharBstr(bstr, start, len_var) -> BSTR (pops 12 bytes)
                let bstr = self.mem.read_u32(self.regs.esp);
                let start = self.mem.read_i32(self.regs.esp.wrapping_add(4));
                let len_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let s = self.mem.read_bstr_string(bstr);
                let u16_chars: Vec<u16> = s.encode_utf16().collect();
                let sub = if start > 0 && ((start - 1) as usize) < u16_chars.len() {
                    let start_idx = (start - 1) as usize;
                    let avail = u16_chars.len() - start_idx;
                    let count = if len_ptr != 0 {
                        let lvar = self.mem.read_variant_deref(len_ptr);
                        if lvar.vt != crate::types::variant::VT_EMPTY {
                            (lvar.as_i32().unwrap_or(0).max(0) as usize).min(avail)
                        } else {
                            avail
                        }
                    } else {
                        avail
                    };
                    String::from_utf16_lossy(&u16_chars[start_idx..start_idx + count])
                } else {
                    String::new()
                };
                let new_bstr = self.mem.alloc_bstr_from_str(&sub);
                self.regs.eax = new_bstr;
            }
            632 => {
                // rtcMidCharVar(dst, str_var, start, len_var) (pops 16 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let str_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let start = self.mem.read_i32(self.regs.esp.wrapping_add(8));
                let len_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                self.regs.esp = self.regs.esp.wrapping_add(16);
                let var = self.mem.read_variant_deref(str_ptr);
                let s = self.variant_to_string(&var);
                let u16_chars: Vec<u16> = s.encode_utf16().collect();
                let sub = if start > 0 && ((start - 1) as usize) < u16_chars.len() {
                    let start_idx = (start - 1) as usize;
                    let avail = u16_chars.len() - start_idx;
                    let count = if len_ptr != 0 {
                        let lvar = self.mem.read_variant_deref(len_ptr);
                        if lvar.vt != crate::types::variant::VT_EMPTY {
                            (lvar.as_i32().unwrap_or(0).max(0) as usize).min(avail)
                        } else {
                            avail
                        }
                    } else {
                        avail
                    };
                    String::from_utf16_lossy(&u16_chars[start_idx..start_idx + count])
                } else {
                    String::new()
                };
                let new_bstr = self.mem.alloc_bstr_from_str(&sub);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            647 => {
                // rtcCurrentDir(dst, drive) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let _drive_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let cur = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
                let s = cur.to_string_lossy().to_string();
                let new_bstr = self.mem.alloc_bstr_from_str(&s);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            660 => {
                // rtcVarFromFormatVar(dst, val, fmt, fd, fw) (pops 20 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let val_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let fmt_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let _fd_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                let _fw_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(16));
                self.regs.esp = self.regs.esp.wrapping_add(20);
                let val_var = self.mem.read_variant_deref(val_ptr);
                let fmt_var = if fmt_ptr != 0 { self.mem.read_variant_deref(fmt_ptr) } else { crate::types::Variant::empty() };
                let val_str = self.variant_to_string(&val_var);
                let fmt_str = self.variant_to_string(&fmt_var);
                let b_res = unsafe {
                    let v_bstr = crate::types::bstr::sys_alloc_string(&val_str);
                    let f_bstr = crate::types::bstr::sys_alloc_string(&fmt_str);
                    let v_var = crate::types::Variant::from_bstr(v_bstr);
                    let f_var = crate::types::Variant::from_bstr(f_bstr);
                    let res = crate::rtc::rtcBstrFromFormatVar(&v_var, &f_var, 0, 0);
                    crate::types::bstr::SysFreeString(v_bstr);
                    crate::types::bstr::SysFreeString(f_bstr);
                    res
                };
                let formatted_str = unsafe { crate::types::bstr::bstr_to_string(b_res) };
                unsafe { crate::types::bstr::SysFreeString(b_res) };
                let new_bstr = self.mem.alloc_bstr_from_str(&formatted_str);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            666 => {
                // rtcEnvironVar(dst, env) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let env_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let var = self.mem.read_variant_deref(env_ptr);
                let s = if (var.vt & !crate::types::variant::VT_BYREF) == crate::types::VT_I2 || (var.vt & !crate::types::variant::VT_BYREF) == crate::types::VT_I4 {
                    let idx = var.as_i32().unwrap_or(1);
                    std::env::vars().nth((idx - 1).max(0) as usize).map(|(k, v)| format!("{}={}", k, v)).unwrap_or_default()
                } else {
                    let name = self.variant_to_string(&var);
                    std::env::var(&name).unwrap_or_default()
                };
                let new_bstr = self.mem.alloc_bstr_from_str(&s);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            693 => {
                // rtcByteValueBstr(bstr) -> u8 (AscB) (pops 4 bytes)
                let bstr_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let b = if bstr_ptr != 0 { self.mem.read_u8(bstr_ptr) } else { 0 };
                self.regs.eax = b as u32;
            }
            695 => {
                // rtcVarBstrFromByte(dst, byte_val) (ChrB) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let byte_val = self.mem.read_u8(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let new_bstr = self.mem.alloc_bstr(&[byte_val as u16]);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            696 => {
                // rtcCharValueBstr(bstr) -> i16 (AscW) (pops 4 bytes)
                let bstr_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let ch = if bstr_ptr != 0 { self.mem.read_u16(bstr_ptr) } else { 0 };
                self.regs.eax = (ch as i16 as u16) as u32;
            }
            698 => {
                // rtcVarBstrFromChar(dst, ch) (ChrW) (pops 8 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let ch = self.mem.read_u16(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                let new_bstr = self.mem.alloc_bstr(&[ch]);
                let mut out_var = crate::types::Variant::empty();
                out_var.vt = crate::types::variant::VT_BSTR;
                out_var.data[0..4].copy_from_slice(&new_bstr.to_le_bytes());
                self.mem.write_variant(dst_ptr, &out_var);
                self.regs.eax = dst_ptr;
            }
            110 | 387 | 389 => {
                // __vbaRedimVar2 (110) / __vbaRedimPreserveVar (387) / __vbaRedimVar (389) - cdecl (caller pops)
                let vt = self.mem.read_u32(self.regs.esp);
                let p_var = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let _c_dims = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                if ordinal == 387 {
                    let ubound = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                    let lbound = self.mem.read_u32(self.regs.esp.wrapping_add(16)) as i32;
                    let mut arr_var = self.mem.read_variant(p_var);
                    while (arr_var.vt & crate::types::variant::VT_BYREF) != 0 && (arr_var.vt & 0x0FFF) == crate::types::VT_VARIANT {
                        let ref_ptr = u32::from_le_bytes(arr_var.data[..4].try_into().unwrap());
                        if ref_ptr == 0 { break; }
                        arr_var = self.mem.read_variant(ref_ptr);
                    }
                    let sa_ptr = u32::from_le_bytes(arr_var.data[..4].try_into().unwrap());
                    if sa_ptr != 0 {
                        let old_count = self.mem.read_u32(sa_ptr + 16);
                        let old_data_ptr = self.mem.read_u32(sa_ptr + 12);
                        let elem_size = self.mem.read_u32(sa_ptr + 4);
                        let new_count = if ubound >= lbound { (ubound - lbound + 1) as u32 } else { 0 };
                        let new_data_ptr = self.mem.alloc_bytes((new_count * elem_size) as usize);
                        let copy_count = old_count.min(new_count);
                        for i in 0..(copy_count * elem_size) {
                            let b = self.mem.read_u8(old_data_ptr.wrapping_add(i));
                            self.mem.write_u8(new_data_ptr.wrapping_add(i), b);
                        }
                        self.mem.write_u32(sa_ptr + 12, new_data_ptr);
                        self.mem.write_u32(sa_ptr + 16, new_count);
                        self.mem.write_u32(sa_ptr + 20, lbound as u32);
                    } else {
                        let count = if ubound >= lbound { (ubound - lbound + 1) as u32 } else { 0 };
                        let elem_size = if (vt & 0x0FFF) == (crate::types::VT_VARIANT as u32) { 16 } else { 4 };
                        let data_size = (count * elem_size) as usize;
                        let data_ptr = self.mem.alloc_bytes(data_size);
                        let sa_ptr = self.mem.alloc_bytes(24);
                        self.mem.write_u16(sa_ptr, 1);
                        self.mem.write_u16(sa_ptr + 2, 0x0800);
                        self.mem.write_u32(sa_ptr + 4, elem_size);
                        self.mem.write_u32(sa_ptr + 8, 0);
                        self.mem.write_u32(sa_ptr + 12, data_ptr);
                        self.mem.write_u32(sa_ptr + 16, count);
                        self.mem.write_u32(sa_ptr + 20, lbound as u32);

                        let mut var = crate::types::Variant::empty();
                        var.vt = (vt as u16) | crate::types::variant::VT_ARRAY;
                        var.data[0..4].copy_from_slice(&sa_ptr.to_le_bytes());
                        self.mem.write_variant(p_var, &var);
                    }
                } else if ordinal == 389 {
                    let ubound = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                    let lbound = self.mem.read_u32(self.regs.esp.wrapping_add(16)) as i32;
                    let count = if ubound >= lbound { (ubound - lbound + 1) as u32 } else { 0 };
                    let elem_size = if (vt & 0x0FFF) == (crate::types::VT_VARIANT as u32) { 16 } else { 4 };
                    let data_size = (count * elem_size) as usize;
                    let data_ptr = self.mem.alloc_bytes(data_size);
                    let sa_ptr = self.mem.alloc_bytes(24);
                    self.mem.write_u16(sa_ptr, 1); // c_dims = 1
                    self.mem.write_u16(sa_ptr + 2, 0x0800); // FADF_VARIANT
                    self.mem.write_u32(sa_ptr + 4, elem_size);
                    self.mem.write_u32(sa_ptr + 8, 0); // locks
                    self.mem.write_u32(sa_ptr + 12, data_ptr);
                    self.mem.write_u32(sa_ptr + 16, count);
                    self.mem.write_u32(sa_ptr + 20, lbound as u32);

                    let mut var = crate::types::Variant::empty();
                    var.vt = (vt as u16) | crate::types::variant::VT_ARRAY;
                    var.data[0..4].copy_from_slice(&sa_ptr.to_le_bytes());
                    self.mem.write_variant(p_var, &var);
                } else {
                    let ubound2 = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                    let lbound2 = self.mem.read_u32(self.regs.esp.wrapping_add(16)) as i32;
                    let ubound1 = self.mem.read_u32(self.regs.esp.wrapping_add(20)) as i32;
                    let lbound1 = self.mem.read_u32(self.regs.esp.wrapping_add(24)) as i32;
                    let count1 = if ubound1 >= lbound1 { (ubound1 - lbound1 + 1) as u32 } else { 0 };
                    let count2 = if ubound2 >= lbound2 { (ubound2 - lbound2 + 1) as u32 } else { 0 };
                    let elem_size = if (vt & 0x0FFF) == (crate::types::VT_VARIANT as u32) { 16 } else { 4 };
                    let total_count = count1 * count2;
                    let data_size = (total_count * elem_size) as usize;
                    let data_ptr = self.mem.alloc_bytes(data_size);
                    let sa_ptr = self.mem.alloc_bytes(32);
                    self.mem.write_u16(sa_ptr, 2);
                    self.mem.write_u16(sa_ptr + 2, 0x0800);
                    self.mem.write_u32(sa_ptr + 4, elem_size);
                    self.mem.write_u32(sa_ptr + 8, 0);
                    self.mem.write_u32(sa_ptr + 12, data_ptr);
                    self.mem.write_u32(sa_ptr + 16, count2);
                    self.mem.write_u32(sa_ptr + 20, lbound2 as u32);
                    self.mem.write_u32(sa_ptr + 24, count1);
                    self.mem.write_u32(sa_ptr + 28, lbound1 as u32);

                    let mut var = crate::types::Variant::empty();
                    var.vt = (vt as u16) | crate::types::variant::VT_ARRAY;
                    var.data[0..4].copy_from_slice(&sa_ptr.to_le_bytes());
                    self.mem.write_variant(p_var, &var);
                }
                self.regs.eax = 0;
            }
            471 => {
                // __vbaVarIndexStore - cdecl (caller pops)
                let p_arr_var = self.mem.read_u32(self.regs.esp);
                let num_indices = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let val_ptr = self.regs.esp.wrapping_add(8);
                let idx1_var = self.mem.read_variant_deref(self.regs.esp.wrapping_add(24));
                let idx1 = idx1_var.as_i32().unwrap_or(0);

                let mut arr_var = self.mem.read_variant(p_arr_var);
                while (arr_var.vt & crate::types::variant::VT_BYREF) != 0 && (arr_var.vt & 0x0FFF) == crate::types::VT_VARIANT {
                    let ref_ptr = u32::from_le_bytes(arr_var.data[..4].try_into().unwrap());
                    if ref_ptr == 0 { break; }
                    arr_var = self.mem.read_variant(ref_ptr);
                }
                let sa_ptr = u32::from_le_bytes(arr_var.data[..4].try_into().unwrap());
                if sa_ptr != 0 {
                    let c_dims = self.mem.read_u16(sa_ptr);
                    let cb_elements = self.mem.read_u32(sa_ptr + 4);
                    let pv_data = self.mem.read_u32(sa_ptr + 12);
                    if c_dims == 1 {
                        let c_elements = self.mem.read_u32(sa_ptr + 16) as i32;
                        let l_lbound = self.mem.read_u32(sa_ptr + 20) as i32;
                        if idx1 < l_lbound || idx1 >= l_lbound + c_elements {
                            self.handle_error(9)?; // Subscript out of range
                        } else {
                            let offset = (idx1 - l_lbound) as u32;
                            let elem_addr = pv_data.wrapping_add(offset * cb_elements);
                            for i in 0..cb_elements {
                                let b = self.mem.read_u8(val_ptr.wrapping_add(i));
                                self.mem.write_u8(elem_addr.wrapping_add(i), b);
                            }
                        }
                    } else if c_dims == 2 && num_indices >= 2 {
                        let idx2_var = self.mem.read_variant_deref(self.regs.esp.wrapping_add(40));
                        let idx2 = idx2_var.as_i32().unwrap_or(0);
                        let c_elements2 = self.mem.read_u32(sa_ptr + 16) as i32;
                        let l_lbound2 = self.mem.read_u32(sa_ptr + 20) as i32;
                        let c_elements1 = self.mem.read_u32(sa_ptr + 24) as i32;
                        let l_lbound1 = self.mem.read_u32(sa_ptr + 28) as i32;
                        if idx1 < l_lbound1 || idx1 >= l_lbound1 + c_elements1 || idx2 < l_lbound2 || idx2 >= l_lbound2 + c_elements2 {
                            self.handle_error(9)?;
                        } else {
                            let offset = ((idx1 - l_lbound1) * c_elements2 + (idx2 - l_lbound2)) as u32;
                            let elem_addr = pv_data.wrapping_add(offset * cb_elements);
                            for i in 0..cb_elements {
                                let b = self.mem.read_u8(val_ptr.wrapping_add(i));
                                self.mem.write_u8(elem_addr.wrapping_add(i), b);
                            }
                        }
                    }
                }
                self.regs.eax = 0;
            }
            468 => {
                // __vbaVarIndexLoad - cdecl (caller pops)
                let dst_var = self.mem.read_u32(self.regs.esp);
                let p_arr_var = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let num_indices = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let idx1_var = self.mem.read_variant_deref(self.regs.esp.wrapping_add(12));
                let idx1 = idx1_var.as_i32().unwrap_or(0);

                let mut arr_var = self.mem.read_variant(p_arr_var);
                while (arr_var.vt & crate::types::variant::VT_BYREF) != 0 && (arr_var.vt & 0x0FFF) == crate::types::VT_VARIANT {
                    let ref_ptr = u32::from_le_bytes(arr_var.data[..4].try_into().unwrap());
                    if ref_ptr == 0 { break; }
                    arr_var = self.mem.read_variant(ref_ptr);
                }
                let sa_ptr = u32::from_le_bytes(arr_var.data[..4].try_into().unwrap());
                if sa_ptr != 0 {
                    let c_dims = self.mem.read_u16(sa_ptr);
                    let cb_elements = self.mem.read_u32(sa_ptr + 4);
                    let pv_data = self.mem.read_u32(sa_ptr + 12);
                    if c_dims == 1 {
                        let c_elements = self.mem.read_u32(sa_ptr + 16) as i32;
                        let l_lbound = self.mem.read_u32(sa_ptr + 20) as i32;
                        if idx1 < l_lbound || idx1 >= l_lbound + c_elements {
                            self.handle_error(9)?;
                        } else {
                            let offset = (idx1 - l_lbound) as u32;
                            let elem_addr = pv_data.wrapping_add(offset * cb_elements);
                            if cb_elements == 16 {
                                for i in 0..16 {
                                    let b = self.mem.read_u8(elem_addr.wrapping_add(i));
                                    self.mem.write_u8(dst_var.wrapping_add(i), b);
                                }
                            } else if cb_elements == 4 {
                                let val = self.mem.read_u32(elem_addr);
                                let mut out = crate::types::Variant::empty();
                                out.vt = crate::types::VT_I4;
                                out.data[..4].copy_from_slice(&val.to_le_bytes());
                                self.mem.write_variant(dst_var, &out);
                            }
                        }
                    } else if c_dims == 2 && num_indices >= 2 {
                        let idx2_var = self.mem.read_variant_deref(self.regs.esp.wrapping_add(28));
                        let idx2 = idx2_var.as_i32().unwrap_or(0);
                        let c_elements2 = self.mem.read_u32(sa_ptr + 16) as i32;
                        let l_lbound2 = self.mem.read_u32(sa_ptr + 20) as i32;
                        let c_elements1 = self.mem.read_u32(sa_ptr + 24) as i32;
                        let l_lbound1 = self.mem.read_u32(sa_ptr + 28) as i32;
                        if idx1 < l_lbound1 || idx1 >= l_lbound1 + c_elements1 || idx2 < l_lbound2 || idx2 >= l_lbound2 + c_elements2 {
                            self.handle_error(9)?;
                        } else {
                            let offset = ((idx1 - l_lbound1) * c_elements2 + (idx2 - l_lbound2)) as u32;
                            let elem_addr = pv_data.wrapping_add(offset * cb_elements);
                            if cb_elements == 16 {
                                for i in 0..16 {
                                    let b = self.mem.read_u8(elem_addr.wrapping_add(i));
                                    self.mem.write_u8(dst_var.wrapping_add(i), b);
                                }
                            } else if cb_elements == 4 {
                                let val = self.mem.read_u32(elem_addr);
                                let mut out = crate::types::Variant::empty();
                                out.vt = crate::types::VT_I4;
                                out.data[..4].copy_from_slice(&val.to_le_bytes());
                                self.mem.write_variant(dst_var, &out);
                            }
                        }
                    }
                }
                self.regs.eax = dst_var;
            }
            278 => {
                // __vbaInStrB(compare: i32, needle: Bstr, haystack: Bstr, start: i32) -> i32 (stdcall, pops 16 bytes)
                let _compare = self.mem.read_u32(self.regs.esp) as i32;
                let needle_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let hay_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let start = self.mem.read_u32(self.regs.esp.wrapping_add(12)) as i32;
                self.regs.esp = self.regs.esp.wrapping_add(16);
                if hay_ptr == 0 || needle_ptr == 0 {
                    self.regs.eax = 0;
                } else {
                    let hay_len = self.mem.read_u32(hay_ptr.wrapping_sub(4)) as usize;
                    let needle_len = self.mem.read_u32(needle_ptr.wrapping_sub(4)) as usize;
                    if needle_len == 0 {
                        self.regs.eax = start.max(1) as u32;
                    } else {
                        let start_idx = (start.max(1) - 1) as usize;
                        if start_idx + needle_len > hay_len {
                            self.regs.eax = 0;
                        } else {
                            let mut found = 0;
                            for i in start_idx..=(hay_len - needle_len) {
                                let mut m = true;
                                for j in 0..needle_len {
                                    if self.mem.read_u8(hay_ptr.wrapping_add((i + j) as u32)) != self.mem.read_u8(needle_ptr.wrapping_add(j as u32)) {
                                        m = false;
                                        break;
                                    }
                                }
                                if m {
                                    found = i + 1;
                                    break;
                                }
                            }
                            self.regs.eax = found as u32;
                        }
                    }
                }
            }
            280 => {
                // __vbaInStrVarB(dst, comp, needle, haystack, start) -> dst in EAX (pops 20 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let _comp = self.mem.read_i32(self.regs.esp.wrapping_add(4));
                let needle_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let hay_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                let start = self.mem.read_i32(self.regs.esp.wrapping_add(16));
                self.regs.esp = self.regs.esp.wrapping_add(20);
                let needle_var = if needle_ptr != 0 { self.mem.read_variant_deref(needle_ptr) } else { crate::types::Variant::empty() };
                let hay_var = if hay_ptr != 0 { self.mem.read_variant_deref(hay_ptr) } else { crate::types::Variant::empty() };
                let s1 = self.variant_to_string(&hay_var);
                let s2 = self.variant_to_string(&needle_var);
                let hay_bytes = s1.as_bytes();
                let needle_bytes = s2.as_bytes();
                let start_pos = start.max(1);
                let idx = if needle_bytes.is_empty() {
                    start_pos
                } else {
                    let start_idx = (start_pos - 1) as usize;
                    if start_idx + needle_bytes.len() > hay_bytes.len() {
                        0
                    } else {
                        hay_bytes[start_idx..]
                            .windows(needle_bytes.len())
                            .position(|w| w == needle_bytes)
                            .map(|p| (start_idx + p + 1) as i32)
                            .unwrap_or(0)
                    }
                };
                let out = crate::types::Variant::from_i32(idx);
                self.mem.write_variant(dst_ptr, &out);
                self.regs.eax = dst_ptr;
            }
            335 | 336 => {
                // __vbaMidStmtBstr (335) / __vbaMidStmtBstrB (336) (stdcall, pops 20 bytes)
                // Stack: [esp+0]=flags, [esp+4]=src_bstr, [esp+8]=len, [esp+12]=start, [esp+16]=dst_ptr
                let src_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let len = self.mem.read_i32(self.regs.esp.wrapping_add(8));
                let start = self.mem.read_i32(self.regs.esp.wrapping_add(12));
                let dst_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(16));
                self.regs.esp = self.regs.esp.wrapping_add(20);
                if dst_ptr != 0 && src_ptr != 0 && start > 0 {
                    let bstr_dst = self.mem.read_u32(dst_ptr);
                    if bstr_dst != 0 {
                        if ordinal == 336 {
                            // byte-based
                            let dst_bytes = self.mem.read_u32(bstr_dst.wrapping_sub(4)) as usize;
                            let src_bytes = self.mem.read_u32(src_ptr.wrapping_sub(4)) as usize;
                            let start_idx = (start - 1) as usize;
                            if start_idx < dst_bytes {
                                let avail = dst_bytes - start_idx;
                                let to_copy = if len < 0 { src_bytes.min(avail) } else { (len as usize).min(src_bytes).min(avail) };
                                for i in 0..to_copy {
                                    let b = self.mem.read_u8(src_ptr.wrapping_add(i as u32));
                                    self.mem.write_u8(bstr_dst.wrapping_add((start_idx + i) as u32), b);
                                }
                            }
                        } else {
                            // char-based (UTF-16)
                            let dst_chars = (self.mem.read_u32(bstr_dst.wrapping_sub(4)) / 2) as usize;
                            let src_chars = (self.mem.read_u32(src_ptr.wrapping_sub(4)) / 2) as usize;
                            let start_idx = (start - 1) as usize;
                            if start_idx < dst_chars {
                                let avail = dst_chars - start_idx;
                                let to_copy = if len < 0 { src_chars.min(avail) } else { (len as usize).min(src_chars).min(avail) };
                                for i in 0..to_copy {
                                    let ch = self.mem.read_u16(src_ptr.wrapping_add((i * 2) as u32));
                                    self.mem.write_u16(bstr_dst.wrapping_add(((start_idx + i) * 2) as u32), ch);
                                }
                            }
                        }
                    }
                }
            }
            337 => {
                // __vbaMidStmtVar (stdcall, pops 16 bytes)
                // Stack: [esp+0]=src_ptr, [esp+4]=len, [esp+8]=start, [esp+12]=dst_ptr
                let src_ptr = self.mem.read_u32(self.regs.esp);
                let len = self.mem.read_i32(self.regs.esp.wrapping_add(4));
                let start = self.mem.read_i32(self.regs.esp.wrapping_add(8));
                let dst_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                self.regs.esp = self.regs.esp.wrapping_add(16);
                if dst_ptr != 0 && src_ptr != 0 && start > 0 {
                    let v_dst = self.mem.read_variant_deref(dst_ptr);
                    let v_src = self.mem.read_variant_deref(src_ptr);
                    let s_dst = self.variant_to_string(&v_dst);
                    let s_src = self.variant_to_string(&v_src);
                    let mut u_dst: Vec<u16> = s_dst.encode_utf16().collect();
                    let u_src: Vec<u16> = s_src.encode_utf16().collect();
                    let start_idx = (start - 1) as usize;
                    if start_idx < u_dst.len() {
                        let avail = u_dst.len() - start_idx;
                        let count = if len < 0 { u_src.len().min(avail) } else { (len as usize).min(u_src.len()).min(avail) };
                        u_dst[start_idx..(start_idx + count)].copy_from_slice(&u_src[..count]);
                        let new_bstr = self.mem.alloc_bstr(&u_dst);
                        let mut out = crate::types::Variant::empty();
                        out.vt = crate::types::VT_BSTR;
                        out.data[..4].copy_from_slice(&new_bstr.to_le_bytes());
                        self.mem.write_variant(dst_ptr, &out);
                    }
                }
            }
            390 => {
                // __vbaRefVarAry(p_var: *mut Variant) -> *mut *mut SafeArray in EAX (pops 4 bytes)
                let p_var = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if p_var != 0 {
                    let var = self.mem.read_variant(p_var);
                    if (var.vt & crate::types::variant::VT_BYREF) != 0 {
                        self.regs.eax = u32::from_le_bytes(var.data[..4].try_into().unwrap());
                    } else {
                        self.regs.eax = p_var.wrapping_add(8);
                    }
                } else {
                    self.regs.eax = 0;
                }
            }
            408 => {
                // __vbaStrCompVar(dst, comp, s2, s1) -> dst in EAX (pops 16 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let comp = self.mem.read_i32(self.regs.esp.wrapping_add(4));
                let s2_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                let s1_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                self.regs.esp = self.regs.esp.wrapping_add(16);
                let s1_var = if s1_ptr != 0 { self.mem.read_variant_deref(s1_ptr) } else { crate::types::Variant::empty() };
                let s2_var = if s2_ptr != 0 { self.mem.read_variant_deref(s2_ptr) } else { crate::types::Variant::empty() };
                let str1 = self.variant_to_string(&s1_var);
                let str2 = self.variant_to_string(&s2_var);
                let res = if comp == 1 {
                    str1.to_lowercase().cmp(&str2.to_lowercase())
                } else {
                    str1.cmp(&str2)
                };
                let code: i16 = match res {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                self.mem.write_variant(dst_ptr, &crate::types::Variant::from_i16(code));
                self.regs.eax = dst_ptr;
            }
            475 => {
                // __vbaVarLikeVar(dst, pattern, string) -> dst in EAX (pops 12 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let pat_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let str_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let pat_var = if pat_ptr != 0 { self.read_variant_for_host(pat_ptr) } else { crate::types::Variant::empty() };
                let str_var = if str_ptr != 0 { self.read_variant_for_host(str_ptr) } else { crate::types::Variant::empty() };
                let mut out = crate::types::Variant::empty();
                unsafe {
                    crate::vba::__vbaVarLike(&mut out, &pat_var, &str_var);
                }
                self.mem.write_variant(dst_ptr, &out);
                self.regs.eax = dst_ptr;
            }
            563 => {
                // rtcVarType(var: *const Variant) -> i32 in EAX (stdcall, pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if var_ptr != 0 {
                    let var = self.mem.read_variant_deref(var_ptr);
                    self.regs.eax = (var.vt & 0x2FFF) as u32;
                } else {
                    self.regs.eax = 0;
                }
            }
            713 => {
                // rtcStrReverse(s: Bstr) -> Bstr in EAX (stdcall, pops 4 bytes)
                let bstr_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                let mut chars = self.mem.read_bstr(bstr_ptr);
                chars.reverse();
                self.regs.eax = self.mem.alloc_bstr(&chars);
            }
            644 => {
                // VarPtr(ptr: u32) -> u32 in EAX (stdcall, pops 4 bytes)
                let ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                self.regs.eax = ptr;
            }
            348 => {
                // __vbaObjIs(obj1: u32, obj2: u32) -> i16 in AX (stdcall, pops 8 bytes)
                let obj1 = self.mem.read_u32(self.regs.esp);
                let obj2 = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                self.regs.esp = self.regs.esp.wrapping_add(8);
                self.regs.eax = if obj1 == obj2 { 0xFFFF } else { 0 };
            }
            351 => {
                // __vbaObjVar(var_ptr: *const Variant) -> u32 in EAX (stdcall, pops 4 bytes)
                let var_ptr = self.mem.read_u32(self.regs.esp);
                self.regs.esp = self.regs.esp.wrapping_add(4);
                if var_ptr != 0 {
                    let var = self.mem.read_variant_deref(var_ptr);
                    let vt = var.vt & 0x0FFF;
                    if vt == crate::types::VT_DISPATCH || vt == crate::types::VT_UNKNOWN {
                        self.regs.eax = u32::from_le_bytes(var.data[0..4].try_into().unwrap());
                    } else {
                        self.regs.eax = 0;
                    }
                } else {
                    self.regs.eax = 0;
                }
            }
            625 | 716 => {
                // rtcCreateObject(dst_ptr, prog_id) (625, pops 8) / rtcCreateObject2(dst_ptr, prog_id, server_name) (716, pops 12)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let prog_id_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let server_ptr = if ordinal == 716 {
                    let s = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                    self.regs.esp = self.regs.esp.wrapping_add(12);
                    s
                } else {
                    self.regs.esp = self.regs.esp.wrapping_add(8);
                    0
                };
                let prog_id_str = if prog_id_ptr != 0 {
                    self.mem.read_bstr_string(prog_id_ptr)
                } else {
                    String::new()
                };
                let server_str = if server_ptr != 0 {
                    self.mem.read_bstr_string(server_ptr)
                } else {
                    String::new()
                };
                if !server_str.trim().is_empty() {
                    if dst_ptr != 0 {
                        self.mem.write_variant(dst_ptr, &crate::types::Variant::empty());
                    }
                    self.regs.eax = dst_ptr;
                    self.handle_error(429)?;
                } else {
                    crate::err::clear_vb_error();
                    let host_prog_id = crate::types::bstr::sys_alloc_string(&prog_id_str);
                    let mut out_var = crate::types::Variant::empty();
                    unsafe {
                        crate::rtc::rtcCreateObject2(&mut out_var, host_prog_id, core::ptr::null_mut());
                        crate::types::bstr::SysFreeString(host_prog_id);
                    }
                    if dst_ptr != 0 {
                        self.write_host_variant(dst_ptr, &out_var);
                    }
                    self.regs.eax = dst_ptr;
                    let err = crate::err::get_vb_error_number();
                    if err != 0 {
                        crate::err::clear_vb_error();
                        self.handle_error(err as u32)?;
                    }
                }
            }
            626 => {
                // rtcGetObject(dst_ptr, path_var_ptr, class_var_ptr) -> dst_ptr in EAX (stdcall, pops 12 bytes)
                let dst_ptr = self.mem.read_u32(self.regs.esp);
                let path_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                let class_ptr = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                self.regs.esp = self.regs.esp.wrapping_add(12);
                let mut host_path = if path_ptr != 0 {
                    self.read_variant_for_host(path_ptr)
                } else {
                    crate::types::Variant::empty()
                };
                let mut host_class = if class_ptr != 0 {
                    self.read_variant_for_host(class_ptr)
                } else {
                    crate::types::Variant::empty()
                };
                crate::err::clear_vb_error();
                let mut out_var = crate::types::Variant::empty();
                unsafe {
                    crate::rtc::rtcGetObject(&mut out_var, &mut host_path, &mut host_class);
                }
                if dst_ptr != 0 {
                    self.write_host_variant(dst_ptr, &out_var);
                }
                self.regs.eax = dst_ptr;
                let err = crate::err::get_vb_error_number();
                if err != 0 {
                    crate::err::clear_vb_error();
                    self.handle_error(err as u32)?;
                }
            }
            292 | 293 | 294 | 323 => {
                // __vbaLateMemCall (292), __vbaLateMemCallLd (293), __vbaLateMemCallSt (294), __vbaLateMemSt (323)
                // cdecl (caller cleans stack)
                let (ret_var_ptr, p_obj, member_ptr, c_args, args_base) = if ordinal == 293 {
                    let r = self.mem.read_u32(self.regs.esp);
                    let o = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                    let m = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                    let c = self.mem.read_u32(self.regs.esp.wrapping_add(12));
                    (r, o, m, c, self.regs.esp.wrapping_add(16))
                } else {
                    let o = self.mem.read_u32(self.regs.esp);
                    let m = self.mem.read_u32(self.regs.esp.wrapping_add(4));
                    let c = self.mem.read_u32(self.regs.esp.wrapping_add(8));
                    (0, o, m, c, self.regs.esp.wrapping_add(12))
                };
                let mut member_utf16: Vec<u16> = Vec::new();
                if member_ptr != 0 {
                    let mut cur = member_ptr;
                    for _ in 0..256 {
                        let ch = self.mem.read_u16(cur);
                        member_utf16.push(ch);
                        if ch == 0 {
                            break;
                        }
                        cur = cur.wrapping_add(2);
                    }
                    if member_utf16.last() != Some(&0) {
                        member_utf16.push(0);
                    }
                }
                let mut host_args = Vec::with_capacity(c_args as usize);
                for i in 0..c_args {
                    let arg_addr = args_base.wrapping_add(i * 16);
                    host_args.push(self.read_variant_for_host(arg_addr));
                }
                let host_obj = self
                    .host_com_objects
                    .get(&p_obj)
                    .copied()
                    .unwrap_or(0) as *mut core::ffi::c_void;
                let w_flags = match ordinal {
                    292 => crate::com::DISPATCH_METHOD,
                    293 => crate::com::DISPATCH_METHOD | crate::com::DISPATCH_PROPERTYGET,
                    _ => crate::com::DISPATCH_PROPERTYPUT,
                };
                crate::err::clear_vb_error();
                let mut ret_val = crate::types::Variant::empty();
                unsafe {
                    crate::com::dispatch_invoke_member(
                        host_obj,
                        if member_utf16.is_empty() {
                            core::ptr::null()
                        } else {
                            member_utf16.as_ptr()
                        },
                        w_flags,
                        if host_args.is_empty() {
                            core::ptr::null_mut()
                        } else {
                            host_args.as_mut_ptr()
                        },
                        c_args,
                        &mut ret_val,
                    );
                }
                if ordinal == 293 {
                    if ret_var_ptr != 0 {
                        self.write_host_variant(ret_var_ptr, &ret_val);
                    }
                    self.regs.eax = ret_var_ptr;
                } else {
                    self.regs.eax = 0;
                }
                let err = crate::err::get_vb_error_number();
                if err != 0 {
                    crate::err::clear_vb_error();
                    self.handle_error(err as u32)?;
                }
            }
            _ => {
                self.regs.eax = 0;
            }
        }

        if !self.error_redirected {
            self.regs.eip = ret_addr;
        }
        Ok(())
    }

    /// Registers a 64-bit host `IDispatch` pointer in the 32-bit guest memory space and returns
    /// its 32-bit guest COM object pointer.
    fn register_host_com_object(&mut self, host_ptr: usize) -> u32 {
        if host_ptr == 0 {
            return 0;
        }
        if let Some((&guest_ptr, _)) = self.host_com_objects.iter().find(|(_, &h)| h == host_ptr) {
            return guest_ptr;
        }
        let obj_base = self.mem.alloc_bytes(32);
        let vtbl = self.mem.alloc_bytes(32);
        self.mem.write_u32(vtbl.wrapping_add(0x00), 0xDEAD0000 | 400); // QueryInterface
        self.mem.write_u32(vtbl.wrapping_add(0x04), 0xDEAD0000 | 401); // AddRef
        self.mem.write_u32(vtbl.wrapping_add(0x08), 0xDEAD0000 | 402); // Release
        self.mem.write_u32(vtbl.wrapping_add(0x0C), 0xDEAD0000 | 433); // GetTypeInfoCount
        self.mem.write_u32(vtbl.wrapping_add(0x10), 0xDEAD0000 | 434); // GetTypeInfo
        self.mem.write_u32(vtbl.wrapping_add(0x14), 0xDEAD0000 | 403); // GetIDsOfNames
        self.mem.write_u32(vtbl.wrapping_add(0x18), 0xDEAD0000 | 404); // Invoke
        self.mem.write_u32(obj_base, vtbl);
        self.host_com_objects.insert(obj_base, host_ptr);
        obj_base
    }

    /// Reads a guest Variant argument by value for consumption by host-side runtime helpers.
    ///
    /// Dereferences `VT_BYREF`, strips the `0x8000` temporary marker and re-materialises
    /// guest BSTR payloads as host BSTRs so host helpers never dereference 32-bit guest pointers.
    fn read_variant_for_host(&self, addr: u32) -> crate::types::Variant {
        let mut v = self.mem.read_variant_deref(addr);
        v.vt &= !0x8000;
        if v.vt == crate::types::VT_BSTR {
            let guest = u32::from_le_bytes(v.data[..4].try_into().unwrap());
            let text = self.mem.read_bstr_string(guest);
            let host = crate::types::bstr::sys_alloc_string(&text);
            v = crate::types::Variant::from_bstr(host);
        } else if v.vt == crate::types::VT_DISPATCH || v.vt == crate::types::VT_UNKNOWN {
            let guest_obj = u32::from_le_bytes(v.data[..4].try_into().unwrap());
            let host_ptr = self.host_com_objects.get(&guest_obj).copied().unwrap_or(0);
            v.data = [0u8; 8];
            let ptr_bytes = host_ptr.to_ne_bytes();
            v.data[..ptr_bytes.len()].copy_from_slice(&ptr_bytes);
        }
        v
    }

    /// Writes a host-produced `Variant` into guest memory at `addr`, converting any
    /// host `VT_BSTR` payload into a 32-bit guest BSTR allocation.
    fn write_host_variant(&mut self, addr: u32, var: &crate::types::Variant) {
        let base_vt = var.vt & 0x0FFF;
        if base_vt == crate::types::VT_BSTR {
            let text = unsafe {
                crate::types::bstr::bstr_to_string(var.as_bstr().unwrap_or(core::ptr::null_mut()))
            };
            let guest_bstr = self.mem.alloc_bstr_from_str(&text);
            let mut out = crate::types::Variant::empty();
            out.vt = crate::types::VT_BSTR;
            out.data[..4].copy_from_slice(&guest_bstr.to_le_bytes());
            self.mem.write_variant(addr, &out);
        } else if base_vt == crate::types::VT_DISPATCH || base_vt == crate::types::VT_UNKNOWN {
            let mut ptr_bytes = [0u8; core::mem::size_of::<usize>()];
            ptr_bytes.copy_from_slice(&var.data[..core::mem::size_of::<usize>()]);
            let host_ptr = usize::from_ne_bytes(ptr_bytes);
            let guest_obj = self.register_host_com_object(host_ptr);
            let mut out = crate::types::Variant::empty();
            out.vt = base_vt;
            out.data[..4].copy_from_slice(&guest_obj.to_le_bytes());
            self.mem.write_variant(addr, &out);
        } else {
            self.mem.write_variant(addr, var);
        }
    }

    fn variant_to_string(&self, var: &crate::types::Variant) -> String {
        match var.vt & 0x0FFF {
            crate::types::VT_EMPTY | crate::types::VT_NULL => String::new(),
            crate::types::VT_I1 | crate::types::VT_UI1 => format!("{}", var.data[0]),
            crate::types::VT_I2 => {
                let bytes = [var.data[0], var.data[1]];
                format!("{}", i16::from_le_bytes(bytes))
            }
            crate::types::VT_UI2 => {
                let bytes = [var.data[0], var.data[1]];
                format!("{}", u16::from_le_bytes(bytes))
            }
            crate::types::VT_I4 => {
                let bytes: [u8; 4] = var.data[..4].try_into().unwrap();
                format!("{}", i32::from_le_bytes(bytes))
            }
            crate::types::VT_UI4 => {
                let bytes: [u8; 4] = var.data[..4].try_into().unwrap();
                format!("{}", u32::from_le_bytes(bytes))
            }
            crate::types::VT_R4 => {
                let bytes: [u8; 4] = var.data[..4].try_into().unwrap();
                let f = f32::from_le_bytes(bytes);
                format!("{}", f)
            }
            crate::types::VT_R8 => {
                let bytes: [u8; 8] = var.data[..8].try_into().unwrap();
                let f = f64::from_le_bytes(bytes);
                format!("{}", f)
            }
            crate::types::VT_BOOL => {
                let bytes = [var.data[0], var.data[1]];
                if i16::from_le_bytes(bytes) != 0 {
                    "True".to_string()
                } else {
                    "False".to_string()
                }
            }
            crate::types::VT_DATE => {
                let bytes: [u8; 8] = var.data[..8].try_into().unwrap();
                let ole_date = f64::from_ne_bytes(bytes);
                let unix_secs = ((ole_date - 25569.0) * 86400.0).round() as i64;
                unsafe {
                    let mut tm: libc::tm = core::mem::zeroed();
                    libc::gmtime_r(&unix_secs, &mut tm);
                    let has_date = ole_date.abs() >= 1.0;
                    let frac = (ole_date - ole_date.trunc()).abs();
                    let has_time = frac > 1e-6;
                    if has_date && has_time {
                        format!("{}/{}/{} {:02}:{:02}:{:02}", tm.tm_mon + 1, tm.tm_mday, tm.tm_year + 1900, tm.tm_hour, tm.tm_min, tm.tm_sec)
                    } else if has_time {
                        format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
                    } else {
                        format!("{}/{}/{}", tm.tm_mon + 1, tm.tm_mday, tm.tm_year + 1900)
                    }
                }
            }
            crate::types::VT_BSTR => {
                let ptr = u32::from_le_bytes(var.data[..4].try_into().unwrap());
                if ptr != 0 {
                    self.mem.read_bstr_string(ptr)
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        }
    }

    fn variant_to_f64(&self, v: &crate::types::Variant) -> Option<f64> {
        let vt = v.vt & 0x0FFF;
        match vt {
            crate::types::VT_UI1 | crate::types::VT_I1 => Some(v.data[0] as f64),
            crate::types::VT_I2 => Some(i16::from_le_bytes([v.data[0], v.data[1]]) as f64),
            crate::types::VT_UI2 => Some(u16::from_le_bytes([v.data[0], v.data[1]]) as f64),
            crate::types::VT_I4 => Some(i32::from_le_bytes(v.data[0..4].try_into().unwrap()) as f64),
            crate::types::VT_UI4 => Some(u32::from_le_bytes(v.data[0..4].try_into().unwrap()) as f64),
            crate::types::VT_R4 => Some(f32::from_le_bytes(v.data[0..4].try_into().unwrap()) as f64),
            crate::types::VT_R8 | crate::types::VT_DATE => Some(f64::from_le_bytes(v.data[0..8].try_into().unwrap())),
            crate::types::VT_CY => Some(i64::from_le_bytes(v.data[0..8].try_into().unwrap()) as f64 / 10000.0),
            crate::types::VT_BOOL => Some(if i16::from_le_bytes([v.data[0], v.data[1]]) != 0 { -1.0 } else { 0.0 }),
            crate::types::VT_BSTR => {
                let ptr = u32::from_le_bytes(v.data[0..4].try_into().unwrap());
                if ptr != 0 {
                    let s = self.mem.read_bstr_string(ptr);
                    s.trim().parse::<f64>().ok()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn compare_variants(&self, v1: &crate::types::Variant, v2: &crate::types::Variant, text_compare: bool) -> std::cmp::Ordering {
        if let (Some(f1), Some(f2)) = (self.variant_to_f64(v1), self.variant_to_f64(v2)) {
            return f1.partial_cmp(&f2).unwrap_or(std::cmp::Ordering::Equal);
        }
        let s1 = self.variant_to_string(v1);
        let s2 = self.variant_to_string(v2);
        if text_compare {
            s1.to_lowercase().cmp(&s2.to_lowercase())
        } else {
            s1.cmp(&s2)
        }
    }
}

