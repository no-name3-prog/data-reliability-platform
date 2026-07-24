//! Data quality check definitions and results.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use drp_common::{AssetId, CheckId, RunId, Severity, UtcTimestamp, ValidationStatus};

/// A reusable data-quality check definition.
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
    /// Validator plugin id.
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
}

/// Result of executing a check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Run id for this execution.
    pub run_id: RunId,
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
            check_id,
            status: ValidationStatus::Failed,
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
}
