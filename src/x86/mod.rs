// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Native 32-bit x86 Execution Engine
//!
//! Provides a clean-room 32-bit x86 CPU emulator (`X86Cpu`) supporting instruction decoding,
//! registers (`EAX`..`EDI`, `EIP`, `EFLAGS`), virtual memory with Win32 `FS:[0]` TIB and stack,
//! x87 FPU transcendental math pipeline, and native runtime hook dispatch.

pub mod cpu;
pub mod flags;
pub mod fpu;
pub mod memory;
pub mod registers;

pub use cpu::*;
pub use flags::*;
pub use fpu::*;
pub use memory::*;
pub use registers::*;
