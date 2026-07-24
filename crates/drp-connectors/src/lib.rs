//! Connector plugins. Built and tested only inside Docker.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod mock;
mod registry;

pub use mock::MockConnector;
pub use registry::register_builtin_connectors;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
