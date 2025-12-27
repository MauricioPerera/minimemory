//! Bindings para otros lenguajes de programación.

#[cfg(feature = "python")]
pub mod python;

#[cfg(feature = "nodejs")]
pub mod nodejs;

#[cfg(feature = "ffi")]
pub mod ffi;
