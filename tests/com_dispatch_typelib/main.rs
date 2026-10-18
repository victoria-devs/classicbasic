// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Domain: COM, IDispatch & TypeLib Introspection
//!
//! Subsystems:
//! - Class Factory & ROT: In-Proc server instantiation, Virtual COM Registry, shims
//! - Dispatch & Late-Binding: IDispatch invoke, STA apartments, zombie object lockout
//! - TypeLib & Metadata: MSFT TypeLib parser, ITypeInfo reflection, CallByName
//! - Event Sinks & Collections: IEnumVARIANT dispatch, connection points, iterators

mod class_factory_rot;
mod dispatch_late_binding;
mod typelib_parser;
mod event_sinks_collections;
