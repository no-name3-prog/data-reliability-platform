//! Anomaly detection domain types (engines implement [`crate::AnomalyDetectorPlugin`]).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use drp_common::{AssetId, RunId, Severity, UtcTimestamp};

/// Severity / confidence band for an anomaly finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalySeverity {
    /// Low confidence / informational.
    Low,
    /// Medium confidence.
    Medium,
    /// High confidence.
    High,
    /// Critical operational impact.
    Critical,
}

impl From<AnomalySeverity> for Severity {
    fn from(value: AnomalySeverity) -> Self {
        match value {
            AnomalySeverity::Low => Severity::Info,
            AnomalySeverity::Medium => Severity::Warning,
            AnomalySeverity::High => Severity::Error,
            AnomalySeverity::Critical => Severity::Critical,
        }
    }
}

/// A single anomaly finding produced by a detector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyFinding {
    /// Detector plugin id.
    pub detector: String,
    /// Optional column / field scope.
    #[serde(default)]
    pub field: Option<String>,
    /// Human-readable summary.
    pub message: String,
    /// Severity band.
    pub severity: AnomalySeverity,
    /// Score in `[0, 1]` when applicable (higher = more anomalous).
    #[serde(default)]
    pub score: Option<f64>,
    /// Extra metrics / evidence.
    #[serde(default)]
    pub evidence: IndexMap<String, serde_json::Value>,
}

/// Result of running anomaly detection on an asset sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyReport {
    /// Run id.
    pub run_id: RunId,
    /// Asset analyzed.
    pub asset_id: AssetId,
    /// Detector plugin id.
    pub detector: String,
    /// Findings (empty = healthy).
    pub findings: Vec<AnomalyFinding>,
    /// When detection finished.
    pub finished_at: UtcTimestamp,
}

impl AnomalyReport {
    /// Build an empty healthy report.
    pub fn healthy(asset_id: AssetId, detector: impl Into<String>) -> Self {
        Self {
            run_id: RunId::new(),
            asset_id,
            detector: detector.into(),
            findings: Vec::new(),
            finished_at: UtcTimestamp::now(),
        }
    }

    /// Whether any findings were reported.
    pub fn has_findings(&self) -> bool {
        !self.findings.is_empty()
    }
}
