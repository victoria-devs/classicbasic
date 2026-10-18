// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Domain: Retroactive PE Execution
//!
//! Validates headless execution of compiled VB6 native and p-code executables
//! across all iterations (App01 through App31).

mod common;
mod app01_to_app10_primitives;
mod app11_to_app20_com_quirks;
mod app21_to_app31_abi_pcode;
