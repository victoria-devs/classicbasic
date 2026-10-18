// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Domain: Foundations
//!
//! Subsystems:
//! - Execution Lifecycle: ThunRTMain, VBHeader, PE32 Loader & IAT Resolution
//! - Memory Primitives: VarPtr, GetMem*/PutMem*, Alignment, Win32 Shims
//! - SEH Exception Frames & ErrObject: Dual vtable, thread isolation, unwinding

mod err_object_seh;
mod execution_lifecycle;
mod memory_primitives;
