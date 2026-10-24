// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # cb-x86: 32-bit x86 Hardware Virtual Machine Foundation
//!
//! Provides a clean-room 32-bit x86 CPU emulator (`X86Cpu`) supporting instruction decoding,
//! ModR/M addressing, ALU & EFLAGS updates, x87 FPU stack operations, a 32-bit flat virtual
//! address space (`X86Memory`), Win32 `FS:[0]` Thread Information Block (TIB) emulation,
//! and Structured Exception Handling (SEH) chain unwinding.

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
