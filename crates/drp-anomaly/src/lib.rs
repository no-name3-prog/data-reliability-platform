//! Anomaly detection: sample-based plugins and profile-history engine.
//!
//! # Profile-history engine
//!
//! [`ProfileAnomalyEngine`] compares the latest [`drp_core::DatasetProfile`]
//! with historical runs and detects:
//! - schema changes
//! - row count drops
//! - null spikes
//! - duplicate spikes (unique-ratio drop)
//! - distribution changes
//! - freshness issues
//!
//! Findings can open [`drp_core::Incident`]s via [`AnomalyService`].
//!
//! # Sample plugins
//!
//! Built-ins `null_spike` and `zscore` implement
//! [`drp_core::AnomalyDetectorPlugin`]. Additional detectors register via
//! [`register_builtin_detectors`] or a crate-local `register` helper.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod engine;
mod null_spike;
mod registry;
mod service;
mod zscore;

pub use engine::{ProfileAnomalyEngine, ProfileAnomalyRule, PROFILE_HISTORY_DETECTOR};
pub use null_spike::NullSpikeDetector;
pub use registry::register_builtin_detectors;
pub use service::AnomalyService;
pub use zscore::ZScoreDetector;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
