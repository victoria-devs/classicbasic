// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Domain: Win32 GUI, Windowing & GDI Subsystem (Milestone 3)
//!
//! Subsystems:
//! - Phase 3.1: Win32 Handle Table, Virtual Window Tree & Condvar Message Pump (`USER32` Core)
//! - Phase 3.2: Win32 GDI32 32-Bit Surface, Vector Primitives, BitBlt & Paint Lifecycle
//! - Phase 3.3: Win32 GDI32 Text Output, Font Management & Metrics
//! - Phase 3.4: Win32 ThunderRT6FormDC Form Deserializer & Twips-to-Pixels Engine

mod control_event_sinks;
mod form_deserializer;
mod gdi_surface_primitives;
mod gdi_text_fonts;
mod virtual_windowing;

pub static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
