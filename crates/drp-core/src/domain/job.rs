//! Scheduled / ad-hoc job model.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use drp_common::{JobId, RunId, UtcTimestamp};

/// Lifecycle status of a job run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Queued, not yet started.
    #[default]
    Pending,
    /// Currently executing.
    Running,
    /// Finished successfully.
    Succeeded,
    /// Finished with failure.
    Failed,
    /// Cancelled by user or system.
    Cancelled,
}

/// Definition of a recurring or one-shot job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDefinition {
    /// Unique id.
    pub id: JobId,
    /// Display name.
    pub name: String,
    /// Job handler kind.
    pub kind: String,
    /// Optional schedule expression.
    #[serde(default)]
    pub schedule: Option<String>,
    /// Handler parameters.
    #[serde(default)]
    pub params: IndexMap<String, serde_json::Value>,
    /// Whether the job is active.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Creation time.
    pub created_at: UtcTimestamp,
}

fn default_true() -> bool {
    true
}

impl JobDefinition {
    /// Create a new job definition.
    pub fn new(name: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            id: JobId::new(),
            name: name.into(),
            kind: kind.into(),
            schedule: None,
            params: IndexMap::new(),
            enabled: true,
            created_at: UtcTimestamp::now(),
        }
    }

    /// Set a schedule expression.
    pub fn with_schedule(mut self, schedule: impl Into<String>) -> Self {
        self.schedule = Some(schedule.into());
        self
    }

    /// Attach a parameter.
    pub fn with_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }
}

/// A single execution of a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRun {
    /// Run id.
    pub id: RunId,
    /// Parent job.
    pub job_id: JobId,
    /// Status.
    pub status: JobStatus,
    /// Optional result payload.
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    /// Error message if failed.
    #[serde(default)]
    pub error: Option<String>,
    /// When the run was enqueued.
    pub created_at: UtcTimestamp,
    /// When execution started.
    #[serde(default)]
    pub started_at: Option<UtcTimestamp>,
    /// When execution finished.
    #[serde(default)]
    pub finished_at: Option<UtcTimestamp>,
}

impl JobRun {
    /// Create a pending run for a job.
    pub fn pending(job_id: JobId) -> Self {
        Self {
            id: RunId::new(),
            job_id,
            status: JobStatus::Pending,
            result: None,
            error: None,
            created_at: UtcTimestamp::now(),
            started_at: None,
            finished_at: None,
        }
    }

    /// Mark running.
    pub fn mark_running(&mut self) {
        self.status = JobStatus::Running;
        self.started_at = Some(UtcTimestamp::now());
    }

    /// Mark succeeded.
    pub fn mark_succeeded(&mut self, result: Option<serde_json::Value>) {
        self.status = JobStatus::Succeeded;
        self.result = result;
        self.finished_at = Some(UtcTimestamp::now());
    }

    /// Mark failed.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = JobStatus::Failed;
        self.error = Some(error.into());
        self.finished_at = Some(UtcTimestamp::now());
    }
}
