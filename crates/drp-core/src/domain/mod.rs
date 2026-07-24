//! Core domain entities.
//!
//! Feature engines and plugins share these types. New plugin categories add
//! types here only when the concept is cross-cutting; otherwise keep payload
//! shapes local to the plugin crate.

mod ai;
mod anomaly;
mod asset;
mod check;
mod job;
mod lineage;
mod profile;

pub use ai::{AiMessage, AiRequest, AiResponse, AiRole};
pub use anomaly::{AnomalyFinding, AnomalyReport, AnomalySeverity};
pub use asset::{Asset, ColumnMeta};
pub use check::{CheckDefinition, CheckResult};
pub use job::{JobDefinition, JobRun, JobStatus};
pub use lineage::{LineageEdge, LineageNode};
pub use profile::{ColumnProfile, DatasetProfile};
