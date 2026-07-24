//! Metadata catalog service. Built and tested only inside Docker.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod service;

pub use service::MetadataService;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
