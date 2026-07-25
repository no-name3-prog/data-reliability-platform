//! Shared foundation types for the Data Reliability Platform.
//!
//! # Container-first note
//!
//! This crate is built and tested only inside Docker. See the repository
//! root `Makefile` and `docker-compose.yml`.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod config;
pub mod error;
pub mod id;
pub mod time;
pub mod types;

pub use config::{AnomalyConfig, AppConfig, ConfigError};
pub use error::{Error, Result};
pub use id::{AssetId, CheckId, DatasetId, IncidentId, JobId, PluginId, RunId, TenantId};
pub use time::UtcTimestamp;
pub use types::{
    AssetKind, AssetRef, DataType, HealthStatus, Severity, SourceLocation, ValidationStatus,
};

/// Crate version from Cargo package metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Product name used in logs and API banners.
pub const PRODUCT_NAME: &str = "Data Reliability Platform";
