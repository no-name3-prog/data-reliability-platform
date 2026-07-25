//! Core domain and plugin framework for the Data Reliability Platform.
//!
//! Built and tested only inside Docker containers.
//!
//! # Plugin architecture
//!
//! Extension points are defined as traits in [`plugin`]. Implementations live in
//! sibling crates and register on a [`PluginRegistry`] at process start.
//! See `docs/plugin-architecture.md`.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod events;
pub mod logging;
pub mod platform;
pub mod plugin;

pub use domain::{
    map_sql_type, AiMessage, AiRequest, AiResponse, AiRole, AnomalyFinding, AnomalyKind,
    AnomalyReport, AnomalySeverity, Asset, CatalogDatabase, CatalogSchema, CatalogTable,
    CatalogTree, CheckDefinition, CheckResult, ColumnMeta, ColumnProfile, ColumnProfileDiff,
    DatasetProfile, HistogramBin, Incident, IncidentStatus, JobDefinition, JobRun, JobStatus,
    LineageEdge, LineageNode, ProfileDiff, SemanticType, ValidationRun, ValidationRunStatus,
};
pub use events::{EventBus, PlatformEvent};
pub use logging::init_tracing;
pub use platform::Platform;
pub use plugin::{
    AiProviderPlugin, AnomalyDetectorPlugin, ConnectorPlugin, NotificationPlugin, Plugin,
    PluginBundle, PluginCapability, PluginContext, PluginInfo, PluginRegistry, ProfilerPlugin,
    ValidatorPlugin,
};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
