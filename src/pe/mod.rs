//! Portable Executable (PE32) Loader and Runtime Environment.
//!
//! Provides parsing, virtual memory mapping, base relocation, and import resolution
//! for running legacy 32-bit Windows Visual Basic executables natively on Linux/macOS.

pub mod headers;
pub mod loader;
pub mod ordinals;
pub mod resolver;
pub mod shims;

pub use headers::*;
pub use loader::*;
pub use ordinals::*;
pub use resolver::*;
pub use shims::*;
