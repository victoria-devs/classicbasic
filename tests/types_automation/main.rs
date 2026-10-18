// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Domain: Types & Automation
//!
//! Subsystems:
//! - BSTR: String engine, UTF-16 prefix, null-bytes, in-place Mid$ mutations
//! - VARIANT: VT_* bitmasks, VT_BYREF multi-level dereferencing, algebraic coercion
//! - SafeArray & UDT: Descriptors, locks, strides, multi-dim resizing, UDT structs
//! - Currency & Date: 64-bit 10,000 fixed-point scale, OLE Date 1899-12-30 epoch

mod bstr;
mod currency_date;
mod safearray;
mod variant;
