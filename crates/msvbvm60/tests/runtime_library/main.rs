// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Domain: Runtime Library
//!
//! Subsystems:
//! - Strings & File Channels: Complete rtc* standard string suite, file I/O channels
//! - Math & Financial: Banker's rounding, trigonometric routines, annuities (Pmt, NPV, IRR)
//! - Conversions & Loops: Explicit coercions, overflow traps (Err 6), loop counters

mod conversions;
mod math_financial;
mod strings;
