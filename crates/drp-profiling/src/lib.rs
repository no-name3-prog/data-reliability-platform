//! Data profiling engine.
//!
//! Computes column-level statistics (null %, unique values, min/max/average,
//! histograms) and semantic types (email, phone, date, …). Profiles are stored
//! as history so successive runs can be compared.
//!
//! Built and tested only inside Docker.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod basic;
mod registry;
mod semantic;
mod service;
mod stats;

pub use basic::BasicProfiler;
pub use registry::register_builtin_profilers;
pub use semantic::{detect_semantic_type, semantic_from_physical};
pub use service::ProfilingService;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
