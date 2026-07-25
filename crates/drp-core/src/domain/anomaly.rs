//! Anomaly detection domain types and profile-drift findings.
//!
//! Detectors and the profile-history engine produce [`AnomalyFinding`]s.
//! Incidents are owned by the incident domain module.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use drp_common::{AssetId, IncidentId, RunId, Severity, UtcTimestamp};

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

impl From<Severity> for AnomalySeverity {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Info => AnomalySeverity::Low,
            Severity::Warning => AnomalySeverity::Medium,
            Severity::Error => AnomalySeverity::High,
            Severity::Critical => AnomalySeverity::Critical,
        }
    }
}

/// Classification of what kind of anomaly was detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyKind {
    /// Columns added/removed or type changed between profiles.
    SchemaChange,
    /// Material drop in row count vs baseline / history.
    RowCountDrop,
    /// Null percentage spike on a column.
    NullSpike,
    /// Duplicate / uniqueness regression (unique ratio drop).
    DuplicateSpike,
    /// Distribution shift (mean, stddev, histogram).
    DistributionChange,
    /// Stale data / profile freshness SLA miss.
    Freshness,
    /// Generic / plugin-defined.
    Other,
}

impl AnomalyKind {
    /// Stable machine id used in evidence and incident titles.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SchemaChange => "schema_change",
            Self::RowCountDrop => "row_count_drop",
            Self::NullSpike => "null_spike",
            Self::DuplicateSpike => "duplicate_spike",
            Self::DistributionChange => "distribution_change",
            Self::Freshness => "freshness",
            Self::Other => "other",
        }
    }
}

/// A single anomaly finding produced by a detector or profile rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyFinding {
    /// Detector / rule id (e.g. `profile_history`, `null_spike`).
    pub detector: String,
    /// Kind of anomaly.
    #[serde(default = "default_kind_other")]
    pub kind: AnomalyKind,
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

fn default_kind_other() -> AnomalyKind {
    AnomalyKind::Other
}

impl AnomalyFinding {
    /// Builder for a finding.
    pub fn new(
        detector: impl Into<String>,
        kind: AnomalyKind,
        message: impl Into<String>,
        severity: AnomalySeverity,
    ) -> Self {
        Self {
            detector: detector.into(),
            kind,
            field: None,
            message: message.into(),
            severity,
            score: None,
            evidence: IndexMap::new(),
        }
    }

    /// Attach a field name.
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// Attach a score.
    pub fn with_score(mut self, score: f64) -> Self {
        self.score = Some(score);
        self
    }

    /// Attach evidence.
    pub fn with_evidence(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.evidence.insert(key.into(), value);
        self
    }
}

/// Result of running anomaly detection on an asset (sample or profile history).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyReport {
    /// Run id.
    pub run_id: RunId,
    /// Asset analyzed.
    pub asset_id: AssetId,
    /// Detector / engine id.
    pub detector: String,
    /// Optional baseline profile run used for comparison.
    #[serde(default)]
    pub baseline_run_id: Option<RunId>,
    /// Optional current profile run compared.
    #[serde(default)]
    pub current_run_id: Option<RunId>,
    /// Findings (empty = healthy).
    pub findings: Vec<AnomalyFinding>,
    /// Incident ids opened from this report (when generated).
    #[serde(default)]
    pub incident_ids: Vec<IncidentId>,
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
            baseline_run_id: None,
            current_run_id: None,
            findings: Vec::new(),
            incident_ids: Vec::new(),
            finished_at: UtcTimestamp::now(),
        }
    }

    /// Whether any findings were reported.
    pub fn has_findings(&self) -> bool {
        !self.findings.is_empty()
    }
}
