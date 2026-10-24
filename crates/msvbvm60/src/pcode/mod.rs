// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # P-Code Interpreter Subsystem (ProcCallEngine & MethCallEngine)
//!
//! Re-implementation of the Visual Basic 6.0 P-Code Virtual Machine (VM).
//!
//! P-Code binaries compile procedures into bytecode streams rather than direct x86
//! machine instructions. Procedure entry points are small x86 trampolines:
//! ```asm
//! mov edx, <p_ProcedureDescriptor>
//! mov ecx, <p_ProcCallEngine>
//! jmp ecx
//! ```
//! This module provides:
//! - [`ProcedureDescriptor`]: Parses procedure metadata pointed to by `EDX`.
//! - [`ProcFrame`]: Manages the local variable activation frame and epilogue cleanup.
//! - [`PCodeVM`]: Execution stack machine, arithmetic/relational/bitwise dispatch,
//!   branching, and intrinsic runtime library imports.
//! - Opcode definitions covering `OP_LIT*`, `OP_LD*`, `OP_ST*`, `OP_BRANCH*`, etc.

pub mod descriptor;
pub mod opcodes;
pub mod vm;

pub use descriptor::*;
pub use opcodes::*;
pub use vm::*;
