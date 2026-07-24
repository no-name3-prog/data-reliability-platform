//! Data quality validation engine. Built and tested only inside Docker.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod builtins;
mod registry;
mod service;

pub use builtins::{NotNullValidator, RegexValidator, UniqueValidator};
pub use registry::register_builtin_validators;
pub use service::ValidationService;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
