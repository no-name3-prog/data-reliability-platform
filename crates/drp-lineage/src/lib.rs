//! Lineage graph engine. Built and tested only inside Docker.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod graph;
mod service;

pub use graph::{LineageGraph, LineageSnapshot};
pub use service::LineageService;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
