// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Domain: Win32 GUI, Windowing & GDI Subsystem (Milestone 3)
//!
//! Subsystems:
//! - Phase 3.1: Win32 Handle Table, Virtual Window Tree & Condvar Message Pump (`USER32` Core)

mod virtual_windowing;
