//! Incident management module.
//!
//! Opens incidents from validation failures and anomalies, tracks owners,
//! status, affected assets, full timeline history, and fans out notifications.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod service;

pub use service::IncidentService;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
