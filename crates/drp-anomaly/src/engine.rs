//! Profile-history anomaly engine.
//!
//! Compares the **latest** [`DatasetProfile`] against historical profiles and
//! emits findings for schema changes, row-count drops, null spikes, duplicate
//! spikes, distribution shifts, and freshness SLA misses.
//!
//! # Extending
//!
//! Add a new [`ProfileAnomalyRule`] implementation and register it in
//! [`ProfileAnomalyEngine::with_defaults`].

use chrono::Utc;
use serde_json::json;

use drp_common::AnomalyConfig;
use drp_core::{
    AnomalyFinding, AnomalyKind, AnomalyReport, AnomalySeverity, DatasetProfile, SemanticType,
};

/// Detector id used in findings produced by this engine.
pub const PROFILE_HISTORY_DETECTOR: &str = "profile_history";

/// Rule that inspects current + historical profiles.
pub trait ProfileAnomalyRule: Send + Sync {
    /// Stable rule id.
    fn id(&self) -> &str;
    /// Evaluate and return zero or more findings.
    fn evaluate(
        &self,
        current: &DatasetProfile,
        history: &[DatasetProfile],
        config: &AnomalyConfig,
    ) -> Vec<AnomalyFinding>;
}

/// Orchestrates profile-based anomaly rules.
pub struct ProfileAnomalyEngine {
    rules: Vec<Box<dyn ProfileAnomalyRule>>,
}

impl ProfileAnomalyEngine {
    /// Engine with all built-in rules.
    pub fn with_defaults() -> Self {
        Self {
            rules: vec![
                Box::new(SchemaChangeRule),
                Box::new(RowCountDropRule),
                Box::new(NullSpikeRule),
                Box::new(DuplicateSpikeRule),
                Box::new(DistributionChangeRule),
                Box::new(FreshnessRule),
            ],
        }
    }

    /// Analyze `current` against `history` (newest-first, typically excluding current).
    pub fn analyze(
        &self,
        current: &DatasetProfile,
        history: &[DatasetProfile],
        config: &AnomalyConfig,
    ) -> AnomalyReport {
        let baseline = history.first();
        let mut report = AnomalyReport::healthy(current.asset_id, PROFILE_HISTORY_DETECTOR);
        report.current_run_id = Some(current.run_id);
        report.baseline_run_id = baseline.map(|b| b.run_id);

        for rule in &self.rules {
            let findings = rule.evaluate(current, history, config);
            report.findings.extend(findings);
        }
        report.finished_at = drp_common::UtcTimestamp::now();
        report
    }
}

impl Default for ProfileAnomalyEngine {
    fn default() -> Self {
        Self::with_defaults()
    }
}

fn baseline_of(history: &[DatasetProfile]) -> Option<&DatasetProfile> {
    history.first()
}

// ---------------------------------------------------------------------------
// schema_change
// ---------------------------------------------------------------------------

struct SchemaChangeRule;

impl ProfileAnomalyRule for SchemaChangeRule {
    fn id(&self) -> &str {
        "schema_change"
    }

    fn evaluate(
        &self,
        current: &DatasetProfile,
        history: &[DatasetProfile],
        _config: &AnomalyConfig,
    ) -> Vec<AnomalyFinding> {
        let Some(base) = baseline_of(history) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let cur_names: std::collections::HashSet<_> =
            current.columns.iter().map(|c| c.name.as_str()).collect();
        let base_names: std::collections::HashSet<_> =
            base.columns.iter().map(|c| c.name.as_str()).collect();

        for name in cur_names.difference(&base_names) {
            out.push(
                AnomalyFinding::new(
                    PROFILE_HISTORY_DETECTOR,
                    AnomalyKind::SchemaChange,
                    format!("column '{name}' added since previous profile"),
                    AnomalySeverity::Medium,
                )
                .with_field(*name)
                .with_score(0.7)
                .with_evidence("change", json!("added")),
            );
        }
        for name in base_names.difference(&cur_names) {
            out.push(
                AnomalyFinding::new(
                    PROFILE_HISTORY_DETECTOR,
                    AnomalyKind::SchemaChange,
                    format!("column '{name}' removed since previous profile"),
                    AnomalySeverity::High,
                )
                .with_field(*name)
                .with_score(0.85)
                .with_evidence("change", json!("removed")),
            );
        }
        for col in &current.columns {
            if let Some(b) = base.columns.iter().find(|c| c.name == col.name) {
                if col.data_type != b.data_type {
                    out.push(
                        AnomalyFinding::new(
                            PROFILE_HISTORY_DETECTOR,
                            AnomalyKind::SchemaChange,
                            format!(
                                "column '{}' type changed {:?} → {:?}",
                                col.name, b.data_type, col.data_type
                            ),
                            AnomalySeverity::High,
                        )
                        .with_field(col.name.clone())
                        .with_score(0.9)
                        .with_evidence("from_type", json!(format!("{:?}", b.data_type)))
                        .with_evidence("to_type", json!(format!("{:?}", col.data_type))),
                    );
                }
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// row_count_drop
// ---------------------------------------------------------------------------

struct RowCountDropRule;

impl ProfileAnomalyRule for RowCountDropRule {
    fn id(&self) -> &str {
        "row_count_drop"
    }

    fn evaluate(
        &self,
        current: &DatasetProfile,
        history: &[DatasetProfile],
        config: &AnomalyConfig,
    ) -> Vec<AnomalyFinding> {
        let Some(base) = baseline_of(history) else {
            return Vec::new();
        };
        if base.row_count == 0 {
            return Vec::new();
        }
        if current.row_count >= base.row_count {
            return Vec::new();
        }
        let drop_ratio = (base.row_count as f64 - current.row_count as f64) / base.row_count as f64;
        if drop_ratio + f64::EPSILON < config.row_count_drop_ratio {
            return Vec::new();
        }
        let severity = if drop_ratio >= 0.7 {
            AnomalySeverity::Critical
        } else if drop_ratio >= 0.5 {
            AnomalySeverity::High
        } else {
            AnomalySeverity::Medium
        };
        vec![AnomalyFinding::new(
            PROFILE_HISTORY_DETECTOR,
            AnomalyKind::RowCountDrop,
            format!(
                "row count dropped from {} to {} ({:.1}% drop)",
                base.row_count,
                current.row_count,
                drop_ratio * 100.0
            ),
            severity,
        )
        .with_score(drop_ratio.min(1.0))
        .with_evidence("baseline_rows", json!(base.row_count))
        .with_evidence("current_rows", json!(current.row_count))
        .with_evidence("drop_ratio", json!(drop_ratio))
        .with_evidence("threshold", json!(config.row_count_drop_ratio))]
    }
}

// ---------------------------------------------------------------------------
// null_spike
// ---------------------------------------------------------------------------

struct NullSpikeRule;

impl ProfileAnomalyRule for NullSpikeRule {
    fn id(&self) -> &str {
        "null_spike"
    }

    fn evaluate(
        &self,
        current: &DatasetProfile,
        history: &[DatasetProfile],
        config: &AnomalyConfig,
    ) -> Vec<AnomalyFinding> {
        let Some(base) = baseline_of(history) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for col in &current.columns {
            let Some(b) = base.columns.iter().find(|c| c.name == col.name) else {
                continue;
            };
            let delta = col.null_percentage - b.null_percentage;
            if delta >= config.null_spike_delta {
                let severity = if delta >= 30.0 {
                    AnomalySeverity::High
                } else {
                    AnomalySeverity::Medium
                };
                out.push(
                    AnomalyFinding::new(
                        PROFILE_HISTORY_DETECTOR,
                        AnomalyKind::NullSpike,
                        format!(
                            "column '{}' null % rose by {:.1} points ({:.1}% → {:.1}%)",
                            col.name, delta, b.null_percentage, col.null_percentage
                        ),
                        severity,
                    )
                    .with_field(col.name.clone())
                    .with_score((delta / 100.0).min(1.0))
                    .with_evidence("baseline_null_pct", json!(b.null_percentage))
                    .with_evidence("current_null_pct", json!(col.null_percentage))
                    .with_evidence("delta_points", json!(delta))
                    .with_evidence("threshold_points", json!(config.null_spike_delta)),
                );
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// duplicate_spike (unique_ratio drop)
// ---------------------------------------------------------------------------

struct DuplicateSpikeRule;

impl ProfileAnomalyRule for DuplicateSpikeRule {
    fn id(&self) -> &str {
        "duplicate_spike"
    }

    fn evaluate(
        &self,
        current: &DatasetProfile,
        history: &[DatasetProfile],
        config: &AnomalyConfig,
    ) -> Vec<AnomalyFinding> {
        let Some(base) = baseline_of(history) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for col in &current.columns {
            let Some(b) = base.columns.iter().find(|c| c.name == col.name) else {
                continue;
            };
            // unique_ratio is 0–1; drop means more duplicates.
            let drop = b.unique_ratio - col.unique_ratio;
            if drop >= config.duplicate_unique_ratio_drop {
                out.push(
                    AnomalyFinding::new(
                        PROFILE_HISTORY_DETECTOR,
                        AnomalyKind::DuplicateSpike,
                        format!(
                            "column '{}' unique ratio dropped by {:.2} ({:.2} → {:.2})",
                            col.name, drop, b.unique_ratio, col.unique_ratio
                        ),
                        AnomalySeverity::Medium,
                    )
                    .with_field(col.name.clone())
                    .with_score(drop.min(1.0))
                    .with_evidence("baseline_unique_ratio", json!(b.unique_ratio))
                    .with_evidence("current_unique_ratio", json!(col.unique_ratio))
                    .with_evidence("baseline_distinct", json!(b.distinct_count))
                    .with_evidence("current_distinct", json!(col.distinct_count)),
                );
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// distribution_change
// ---------------------------------------------------------------------------

struct DistributionChangeRule;

impl ProfileAnomalyRule for DistributionChangeRule {
    fn id(&self) -> &str {
        "distribution_change"
    }

    fn evaluate(
        &self,
        current: &DatasetProfile,
        history: &[DatasetProfile],
        config: &AnomalyConfig,
    ) -> Vec<AnomalyFinding> {
        let Some(base) = baseline_of(history) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for col in &current.columns {
            let Some(b) = base.columns.iter().find(|c| c.name == col.name) else {
                continue;
            };
            let (Some(cur_avg), Some(base_avg)) = (col.average, b.average) else {
                continue;
            };
            let scale = b
                .stddev
                .unwrap_or(0.0)
                .max(col.stddev.unwrap_or(0.0))
                .max(1e-9);
            let z = ((cur_avg - base_avg).abs()) / scale;
            if z >= config.distribution_zscore {
                out.push(
                    AnomalyFinding::new(
                        PROFILE_HISTORY_DETECTOR,
                        AnomalyKind::DistributionChange,
                        format!(
                            "column '{}' mean shifted (z≈{z:.2}): {base_avg:.4} → {cur_avg:.4}",
                            col.name
                        ),
                        if z >= config.distribution_zscore * 2.0 {
                            AnomalySeverity::High
                        } else {
                            AnomalySeverity::Medium
                        },
                    )
                    .with_field(col.name.clone())
                    .with_score((z / (config.distribution_zscore * 3.0)).min(1.0))
                    .with_evidence("baseline_avg", json!(base_avg))
                    .with_evidence("current_avg", json!(cur_avg))
                    .with_evidence("z_score", json!(z))
                    .with_evidence("threshold", json!(config.distribution_zscore)),
                );
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// freshness
// ---------------------------------------------------------------------------

struct FreshnessRule;

impl ProfileAnomalyRule for FreshnessRule {
    fn id(&self) -> &str {
        "freshness"
    }

    fn evaluate(
        &self,
        current: &DatasetProfile,
        _history: &[DatasetProfile],
        config: &AnomalyConfig,
    ) -> Vec<AnomalyFinding> {
        let mut out = Vec::new();
        let age_secs = (Utc::now() - current.profiled_at.inner())
            .num_seconds()
            .max(0) as u64;
        if age_secs > config.freshness_max_age_secs {
            out.push(
                AnomalyFinding::new(
                    PROFILE_HISTORY_DETECTOR,
                    AnomalyKind::Freshness,
                    format!(
                        "latest profile is stale (age {age_secs}s > {}s)",
                        config.freshness_max_age_secs
                    ),
                    AnomalySeverity::Medium,
                )
                .with_score(
                    (age_secs as f64 / (config.freshness_max_age_secs as f64 * 2.0)).min(1.0),
                )
                .with_evidence("age_secs", json!(age_secs))
                .with_evidence("max_age_secs", json!(config.freshness_max_age_secs))
                .with_evidence("profiled_at", json!(current.profiled_at.to_rfc3339())),
            );
        }

        // Also flag date/datetime semantic columns whose max is older than SLA when present.
        for col in &current.columns {
            if !matches!(
                col.semantic_type,
                SemanticType::Date | SemanticType::DateTime
            ) {
                continue;
            }
            if let Some(max_v) = &col.max {
                if let Some(s) = max_v.as_str() {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                        let age = (Utc::now() - dt.with_timezone(&Utc)).num_seconds().max(0) as u64;
                        if age > config.freshness_max_age_secs {
                            out.push(
                                AnomalyFinding::new(
                                    PROFILE_HISTORY_DETECTOR,
                                    AnomalyKind::Freshness,
                                    format!(
                                        "column '{}' max timestamp is stale (age {age}s)",
                                        col.name
                                    ),
                                    AnomalySeverity::High,
                                )
                                .with_field(col.name.clone())
                                .with_score(
                                    (age as f64 / (config.freshness_max_age_secs as f64 * 2.0))
                                        .min(1.0),
                                )
                                .with_evidence("max_value", json!(s))
                                .with_evidence("age_secs", json!(age)),
                            );
                        }
                    }
                }
            }
        }
        out
    }
}
