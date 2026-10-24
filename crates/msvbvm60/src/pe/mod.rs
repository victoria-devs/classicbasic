//! Portable Executable (PE32) Loader and Runtime Environment.
//!
//! Provides parsing, virtual memory mapping, base relocation, and import resolution
//! for running legacy 32-bit Windows Visual Basic executables natively on Linux/macOS.

pub mod loader;
pub mod resolver;
pub mod shims;

pub use cb_pe::headers::{self, *};
pub use cb_pe::ordinals::{self, *};
pub use loader::*;
pub use resolver::*;
pub use shims::*;
