//! Core domain entities.
//!
//! Feature engines and plugins share these types. New plugin categories add
//! types here only when the concept is cross-cutting; otherwise keep payload
//! shapes local to the plugin crate.

mod ai;
mod anomaly;
mod asset;
mod catalog;
mod check;
mod job;
mod lineage;
mod profile;

pub use ai::{AiMessage, AiRequest, AiResponse, AiRole};
pub use anomaly::{
    AnomalyFinding, AnomalyKind, AnomalyReport, AnomalySeverity, Incident, IncidentStatus,
};
pub use asset::{Asset, ColumnMeta};
pub use catalog::{map_sql_type, CatalogDatabase, CatalogSchema, CatalogTable, CatalogTree};
pub use check::{CheckDefinition, CheckResult, ValidationRun, ValidationRunStatus};
pub use job::{JobDefinition, JobRun, JobStatus};
pub use lineage::{LineageEdge, LineageNode};
pub use profile::{
    ColumnProfile, ColumnProfileDiff, DatasetProfile, HistogramBin, ProfileDiff, SemanticType,
};
