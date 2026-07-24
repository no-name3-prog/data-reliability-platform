//! HTTP API library. Built and run only inside Docker containers.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod app;
pub mod error;
pub mod metrics;
pub mod routes;
pub mod state;

pub use app::{build_app, build_router};
pub use state::AppState;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
