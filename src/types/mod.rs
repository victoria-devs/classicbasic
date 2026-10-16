//! Visual Basic 6.0 Runtime data structures and COM types.

pub mod bstr;
pub mod currency;
pub mod guid;
pub mod safearray;
pub mod variant;
pub mod vb_header;

pub use bstr::*;
pub use currency::*;
pub use guid::*;
pub use safearray::*;
pub use variant::*;
pub use vb_header::*;

/// Common HRESULT status codes.
pub type HResult = i32;

pub const S_OK: HResult = 0;
pub const S_FALSE: HResult = 1;
pub const E_POINTER: HResult = -2147467261; // 0x80004003
pub const E_INVALIDARG: HResult = -2147024809; // 0x80070057
pub const E_NOINTERFACE: HResult = -2147467262; // 0x80004002
pub const E_FAIL: HResult = -2147467259; // 0x80004005
pub const E_OUTOFMEMORY: HResult = -2147024882; // 0x8007000E
pub const CLASS_E_NOAGGREGATION: HResult = -2147221232; // 0x80040110
pub const CLASS_E_CLASSNOTAVAILABLE: HResult = -2147221231; // 0x80040111
pub const E_NOTIMPL: HResult = -2147467263; // 0x80004001
pub const DISP_E_BADINDEX: HResult = -2147352565; // 0x8002000B
pub const TYPE_E_ELEMENTNOTFOUND: HResult = -2147319765; // 0x8002802B
pub const RPC_E_WRONG_THREAD: HResult = 0x8001010E_u32 as i32; // -2147417842
pub const RPC_E_SERVERCALL_REJECTED: HResult = 0x80010001_u32 as i32; // -2147418111
pub const RPC_E_CALL_REJECTED: HResult = 0x80010001_u32 as i32; // -2147418111
pub const E_UNEXPECTED: HResult = 0x8000FFFF_u32 as i32; // -2147418113
