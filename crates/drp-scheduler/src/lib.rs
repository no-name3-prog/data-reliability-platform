//! Job scheduler. Built and tested only inside Docker.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod handler;
mod service;

pub use handler::{JobHandler, JobHandlerRegistry};
pub use service::SchedulerService;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
