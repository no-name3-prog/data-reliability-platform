//! Core domain and plugin framework for the Data Reliability Platform.
//!
//! Built and tested only inside Docker containers.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod events;
pub mod logging;
pub mod platform;
pub mod plugin;

pub use domain::{
    Asset, CheckDefinition, CheckResult, ColumnMeta, ColumnProfile, DatasetProfile, JobDefinition,
    JobRun, JobStatus, LineageEdge, LineageNode,
};
pub use events::{EventBus, PlatformEvent};
pub use logging::init_tracing;
pub use platform::Platform;
pub use plugin::{
    ConnectorPlugin, NotificationPlugin, Plugin, PluginCapability, PluginContext, PluginInfo,
    PluginRegistry, ProfilerPlugin, ValidatorPlugin,
};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
