//! Incident management domain: severity, status, timeline, owners, affected assets.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use drp_common::{AssetId, CheckId, IncidentId, RunId, UtcTimestamp};

use super::anomaly::{AnomalyKind, AnomalySeverity};

/// Lifecycle of an incident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    /// Newly opened, needs attention.
    #[default]
    Open,
    /// Assigned / being worked.
    InProgress,
    /// Acknowledged by an operator.
    Acknowledged,
    /// Mitigated but monitoring.
    Monitoring,
    /// Closed / fixed.
    Resolved,
}

impl IncidentStatus {
    /// Machine name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Acknowledged => "acknowledged",
            Self::Monitoring => "monitoring",
            Self::Resolved => "resolved",
        }
    }
}

/// What system opened the incident.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IncidentSource {
    /// Data-quality validation failure.
    Validation {
        /// Check id.
        check_id: CheckId,
        /// Check result run id.
        #[serde(default)]
        result_run_id: Option<RunId>,
        /// Validator plugin id.
        #[serde(default)]
        validator: Option<String>,
    },
    /// Anomaly / profile-drift finding.
    Anomaly {
        /// Anomaly report run id.
        report_run_id: RunId,
        /// Detector id.
        #[serde(default)]
        detector: Option<String>,
        /// Finding kind.
        #[serde(default)]
        kind: Option<AnomalyKind>,
    },
    /// Manual / API-created.
    Manual {
        /// Free-form reason.
        #[serde(default)]
        reason: Option<String>,
    },
}

/// Timeline / history event on an incident.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentTimelineEvent {
    /// Event id.
    pub id: RunId,
    /// Parent incident.
    pub incident_id: IncidentId,
    /// When the event occurred.
    pub at: UtcTimestamp,
    /// Actor (user, system, channel).
    #[serde(default)]
    pub actor: Option<String>,
    /// Event type machine name.
    pub event_type: String,
    /// Human-readable summary.
    pub message: String,
    /// Structured details.
    #[serde(default)]
    pub details: IndexMap<String, serde_json::Value>,
}

impl IncidentTimelineEvent {
    /// Create a timeline event.
    pub fn new(
        incident_id: IncidentId,
        event_type: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: RunId::new(),
            incident_id,
            at: UtcTimestamp::now(),
            actor: None,
            event_type: event_type.into(),
            message: message.into(),
            details: IndexMap::new(),
        }
    }

    /// Set actor.
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    /// Attach detail.
    pub fn with_detail(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.details.insert(key.into(), value);
        self
    }
}

/// Full incident record with severity, owner, status, affected assets, and timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    /// Unique incident id.
    pub id: IncidentId,
    /// Primary asset in scope.
    pub asset_id: AssetId,
    /// All affected assets (includes primary).
    #[serde(default)]
    pub affected_assets: Vec<AssetId>,
    /// Source of the incident.
    pub source: IncidentSource,
    /// Severity.
    pub severity: AnomalySeverity,
    /// Short title.
    pub title: String,
    /// Detail message.
    pub message: String,
    /// Lifecycle status.
    pub status: IncidentStatus,
    /// Owner (user email / team id).
    #[serde(default)]
    pub owner: Option<String>,
    /// Optional column / field scope.
    #[serde(default)]
    pub field: Option<String>,
    /// Detector or validator id (denormalized for search).
    #[serde(default)]
    pub detector: Option<String>,
    /// Anomaly kind when applicable.
    #[serde(default)]
    pub kind: Option<AnomalyKind>,
    /// Linked anomaly report run (legacy / denormalized).
    #[serde(default)]
    pub report_run_id: Option<RunId>,
    /// Baseline profile run.
    #[serde(default)]
    pub baseline_run_id: Option<RunId>,
    /// Current profile run.
    #[serde(default)]
    pub current_run_id: Option<RunId>,
    /// Evidence / metrics snapshot.
    #[serde(default)]
    pub evidence: IndexMap<String, serde_json::Value>,
    /// In-memory timeline cache (also stored as history events).
    #[serde(default)]
    pub timeline: Vec<IncidentTimelineEvent>,
    /// Channels notified on open.
    #[serde(default)]
    pub notified_channels: Vec<String>,
    /// Created at.
    pub created_at: UtcTimestamp,
    /// Last update.
    pub updated_at: UtcTimestamp,
}

impl Incident {
    /// Open a new incident from an anomaly finding.
    pub fn from_anomaly_finding(
        asset_id: AssetId,
        report_run_id: RunId,
        baseline_run_id: Option<RunId>,
        current_run_id: Option<RunId>,
        finding: &super::anomaly::AnomalyFinding,
        affected_assets: Vec<AssetId>,
    ) -> Self {
        let now = UtcTimestamp::now();
        let id = IncidentId::new();
        let title = match &finding.field {
            Some(f) => format!("[{}] {} on '{f}'", finding.kind.as_str(), finding.detector),
            None => format!("[{}] {}", finding.kind.as_str(), finding.detector),
        };
        let mut assets = affected_assets;
        if !assets.contains(&asset_id) {
            assets.insert(0, asset_id);
        }
        let created = IncidentTimelineEvent::new(id, "created", "Incident opened from anomaly")
            .with_actor("system:anomaly")
            .with_detail("detector", serde_json::json!(finding.detector))
            .with_detail("kind", serde_json::json!(finding.kind.as_str()));
        Self {
            id,
            asset_id,
            affected_assets: assets,
            source: IncidentSource::Anomaly {
                report_run_id,
                detector: Some(finding.detector.clone()),
                kind: Some(finding.kind),
            },
            severity: finding.severity,
            title,
            message: finding.message.clone(),
            status: IncidentStatus::Open,
            owner: None,
            field: finding.field.clone(),
            detector: Some(finding.detector.clone()),
            kind: Some(finding.kind),
            report_run_id: Some(report_run_id),
            baseline_run_id,
            current_run_id,
            evidence: finding.evidence.clone(),
            timeline: vec![created],
            notified_channels: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    /// Open from a validation failure.
    pub fn from_validation_failure(
        asset_id: AssetId,
        check_id: CheckId,
        result_run_id: RunId,
        validator: impl Into<String>,
        severity: AnomalySeverity,
        title: impl Into<String>,
        message: impl Into<String>,
        affected_assets: Vec<AssetId>,
    ) -> Self {
        let now = UtcTimestamp::now();
        let id = IncidentId::new();
        let validator = validator.into();
        let mut assets = affected_assets;
        if !assets.contains(&asset_id) {
            assets.insert(0, asset_id);
        }
        let created =
            IncidentTimelineEvent::new(id, "created", "Incident opened from validation failure")
                .with_actor("system:validation")
                .with_detail("check_id", serde_json::json!(check_id.to_string()))
                .with_detail("validator", serde_json::json!(validator));
        Self {
            id,
            asset_id,
            affected_assets: assets,
            source: IncidentSource::Validation {
                check_id,
                result_run_id: Some(result_run_id),
                validator: Some(validator.clone()),
            },
            severity,
            title: title.into(),
            message: message.into(),
            status: IncidentStatus::Open,
            owner: None,
            field: None,
            detector: Some(validator),
            kind: None,
            report_run_id: Some(result_run_id),
            baseline_run_id: None,
            current_run_id: None,
            evidence: IndexMap::new(),
            timeline: vec![created],
            notified_channels: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    /// Legacy helper used by older anomaly code paths.
    pub fn from_finding(
        asset_id: AssetId,
        report_run_id: RunId,
        baseline_run_id: Option<RunId>,
        current_run_id: Option<RunId>,
        finding: &super::anomaly::AnomalyFinding,
    ) -> Self {
        Self::from_anomaly_finding(
            asset_id,
            report_run_id,
            baseline_run_id,
            current_run_id,
            finding,
            vec![asset_id],
        )
    }
}
