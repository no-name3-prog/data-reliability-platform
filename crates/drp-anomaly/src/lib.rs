//! Anomaly detector plugins.
//!
//! Built-ins ship here. Additional detectors belong in separate crates that
//! implement [`drp_core::AnomalyDetectorPlugin`] and call
//! [`register`](register_builtin_detectors) or their own `register` helper.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod null_spike;
mod registry;
mod service;
mod zscore;

pub use null_spike::NullSpikeDetector;
pub use registry::register_builtin_detectors;
pub use service::AnomalyService;
pub use zscore::ZScoreDetector;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
