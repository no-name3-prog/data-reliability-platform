//! Data quality check definitions, results, and suite execution history.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use drp_common::{AssetId, CheckId, JobId, RunId, Severity, UtcTimestamp, ValidationStatus};

/// A reusable data-quality check definition.
///
/// Parameters are free-form JSON keyed by the validator plugin id. Built-in
/// validators document their params in `docs/validation.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckDefinition {
    /// Unique id.
    pub id: CheckId,
    /// Human-readable name.
    pub name: String,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Target asset.
    pub asset_id: AssetId,
    /// Validator plugin id (e.g. `not_null`, `regex`, `range`).
    pub validator: String,
    /// Severity when the check fails.
    #[serde(default)]
    pub severity: Severity,
    /// Plugin-specific parameters.
    #[serde(default)]
    pub params: IndexMap<String, serde_json::Value>,
    /// Whether the check is active.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional schedule expression (informational; jobs use [`JobDefinition`]).
    ///
    /// When set via the API, the platform can create/update a `validation` job
    /// that re-runs this check (or its asset suite) on a schedule.
    #[serde(default)]
    pub schedule: Option<String>,
    /// Linked scheduler job id when this check is scheduled.
    #[serde(default)]
    pub job_id: Option<JobId>,
    /// Creation time.
    pub created_at: UtcTimestamp,
}

fn default_true() -> bool {
    true
}

impl CheckDefinition {
    /// Create a new check definition.
    pub fn new(name: impl Into<String>, asset_id: AssetId, validator: impl Into<String>) -> Self {
        Self {
            id: CheckId::new(),
            name: name.into(),
            description: None,
            asset_id,
            validator: validator.into(),
            severity: Severity::Error,
            params: IndexMap::new(),
            enabled: true,
            schedule: None,
            job_id: None,
            created_at: UtcTimestamp::now(),
        }
    }

    /// Set severity.
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Attach a parameter.
    pub fn with_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    /// Attach a schedule expression.
    pub fn with_schedule(mut self, schedule: impl Into<String>) -> Self {
        self.schedule = Some(schedule.into());
        self
    }
}

/// Result of executing a single check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Run id for this individual check execution.
    pub run_id: RunId,
    /// Optional parent suite run (batch / scheduled execution).
    #[serde(default)]
    pub suite_run_id: Option<RunId>,
    /// Check that was executed.
    pub check_id: CheckId,
    /// Outcome status.
    pub status: ValidationStatus,
    /// Severity of the failure (if any).
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// Optional observed metrics.
    #[serde(default)]
    pub metrics: IndexMap<String, serde_json::Value>,
    /// When the check finished.
    pub finished_at: UtcTimestamp,
}

impl CheckResult {
    /// Build a passed result.
    pub fn passed(check_id: CheckId, message: impl Into<String>) -> Self {
        Self {
            run_id: RunId::new(),
            suite_run_id: None,
            check_id,
            status: ValidationStatus::Passed,
            severity: Severity::Info,
            message: message.into(),
            metrics: IndexMap::new(),
            finished_at: UtcTimestamp::now(),
        }
    }

    /// Build a failed result.
    pub fn failed(check_id: CheckId, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            run_id: RunId::new(),
            suite_run_id: None,
            check_id,
            status: ValidationStatus::Failed,
            severity,
            message: message.into(),
            metrics: IndexMap::new(),
            finished_at: UtcTimestamp::now(),
        }
    }

    /// Build an error result (check could not execute cleanly).
    pub fn error(check_id: CheckId, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            run_id: RunId::new(),
            suite_run_id: None,
            check_id,
            status: ValidationStatus::Error,
            severity,
            message: message.into(),
            metrics: IndexMap::new(),
            finished_at: UtcTimestamp::now(),
        }
    }

    /// Attach a metric.
    pub fn with_metric(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metrics.insert(key.into(), value);
        self
    }

    /// Link to a suite run.
    pub fn with_suite_run(mut self, suite_run_id: RunId) -> Self {
        self.suite_run_id = Some(suite_run_id);
        self
    }
}

/// Aggregate status for a suite of checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRunStatus {
    /// All checks passed (or were skipped).
    Passed,
    /// At least one soft failure (warn) and no hard failures.
    Warned,
    /// At least one hard failure.
    Failed,
    /// Suite could not complete (e.g. missing connector).
    Error,
}

impl ValidationRunStatus {
    /// Roll up individual check statuses.
    pub fn from_results(results: &[CheckResult]) -> Self {
        if results.is_empty() {
            return Self::Passed;
        }
        let mut any_warn = false;
        let mut any_fail = false;
        let mut any_error = false;
        for r in results {
            match r.status {
                ValidationStatus::Failed => any_fail = true,
                ValidationStatus::Warned => any_warn = true,
                ValidationStatus::Error => any_error = true,
                ValidationStatus::Passed | ValidationStatus::Skipped => {}
            }
        }
        if any_error && !any_fail {
            return Self::Error;
        }
        if any_fail || any_error {
            return Self::Failed;
        }
        if any_warn {
            return Self::Warned;
        }
        Self::Passed
    }
}

/// One full validation suite execution (saved for every run for history).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRun {
    /// Suite run id.
    pub id: RunId,
    /// Asset under validation (when suite is asset-scoped).
    #[serde(default)]
    pub asset_id: Option<AssetId>,
    /// Optional scheduler job that triggered this run.
    #[serde(default)]
    pub job_id: Option<JobId>,
    /// Connector used for sampling.
    pub connector_id: String,
    /// Aggregate status.
    pub status: ValidationRunStatus,
    /// Individual check results (also stored per-check).
    pub results: Vec<CheckResult>,
    /// Counts for quick history views.
    pub passed: u32,
    /// Failed count.
    pub failed: u32,
    /// Warned count.
    pub warned: u32,
    /// Skipped count.
    pub skipped: u32,
    /// Error count.
    pub errored: u32,
    /// When the suite started.
    pub started_at: UtcTimestamp,
    /// When the suite finished.
    pub finished_at: UtcTimestamp,
}

impl ValidationRun {
    /// Build a suite run from finished check results.
    pub fn from_results(
        asset_id: Option<AssetId>,
        connector_id: impl Into<String>,
        job_id: Option<JobId>,
        started_at: UtcTimestamp,
        results: Vec<CheckResult>,
    ) -> Self {
        let mut passed = 0u32;
        let mut failed = 0u32;
        let mut warned = 0u32;
        let mut skipped = 0u32;
        let mut errored = 0u32;
        for r in &results {
            match r.status {
                ValidationStatus::Passed => passed += 1,
                ValidationStatus::Failed => failed += 1,
                ValidationStatus::Warned => warned += 1,
                ValidationStatus::Skipped => skipped += 1,
                ValidationStatus::Error => errored += 1,
            }
        }
        let status = ValidationRunStatus::from_results(&results);
        Self {
            id: RunId::new(),
            asset_id,
            job_id,
            connector_id: connector_id.into(),
            status,
            results,
            passed,
            failed,
            warned,
            skipped,
            errored,
            started_at,
            finished_at: UtcTimestamp::now(),
        }
    }
}
