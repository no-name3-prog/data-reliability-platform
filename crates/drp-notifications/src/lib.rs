//! Notification channels. Built and tested only inside Docker.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod channels;
mod registry;
mod service;

pub use channels::LogNotifier;
pub use registry::register_builtin_notifiers;
pub use service::NotificationService;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
