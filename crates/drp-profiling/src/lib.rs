//! Data profiling engine. Built and tested only inside Docker.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod basic;
mod registry;
mod service;

pub use basic::BasicProfiler;
pub use registry::register_builtin_profilers;
pub use service::ProfilingService;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
