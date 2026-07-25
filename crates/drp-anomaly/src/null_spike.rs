//! Flags columns whose null ratio exceeds a threshold.

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};

use drp_common::Result;
use drp_core::{
    AnomalyDetectorPlugin, AnomalyFinding, AnomalyKind, AnomalyReport, AnomalySeverity, Asset,
    Plugin, PluginCapability, PluginContext, PluginInfo,
};

/// Detects high null ratios on sampled columns.
pub struct NullSpikeDetector {
    info: PluginInfo,
    default_threshold: f64,
}

impl NullSpikeDetector {
    /// Create with default threshold `0.2` (20% nulls).
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new(
                "null_spike",
                "Null Spike Detector",
                env!("CARGO_PKG_VERSION"),
            )
            .with_description("Flags columns with null ratio above a threshold")
            .with_capability(PluginCapability::AnomalyDetector),
            default_threshold: 0.2,
        }
    }
}

impl Default for NullSpikeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for NullSpikeDetector {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl AnomalyDetectorPlugin for NullSpikeDetector {
    async fn detect(
        &self,
        asset: &Asset,
        rows: &[IndexMap<String, Value>],
        ctx: &PluginContext,
    ) -> Result<AnomalyReport> {
        let threshold = ctx
            .config
            .get("null_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(self.default_threshold);

        let mut report = AnomalyReport::healthy(asset.id, self.info.id.clone());
        if rows.is_empty() {
            return Ok(report);
        }

        let cols: Vec<String> = if asset.columns.is_empty() {
            rows.first()
                .map(|r| r.keys().cloned().collect())
                .unwrap_or_default()
        } else {
            asset.columns.iter().map(|c| c.name.clone()).collect()
        };

        for col in cols {
            let mut nulls = 0u64;
            for row in rows {
                match row.get(&col) {
                    None | Some(Value::Null) => nulls += 1,
                    _ => {}
                }
            }
            let ratio = nulls as f64 / rows.len() as f64;
            if ratio > threshold {
                let mut evidence = IndexMap::new();
                evidence.insert("null_count".into(), json!(nulls));
                evidence.insert("null_ratio".into(), json!(ratio));
                evidence.insert("threshold".into(), json!(threshold));
                report.findings.push(AnomalyFinding {
                    detector: self.info.id.clone(),
                    kind: AnomalyKind::NullSpike,
                    field: Some(col.clone()),
                    message: format!(
                        "column '{col}' null ratio {ratio:.2} exceeds threshold {threshold:.2}"
                    ),
                    severity: if ratio > 0.5 {
                        AnomalySeverity::High
                    } else {
                        AnomalySeverity::Medium
                    },
                    score: Some(ratio.min(1.0)),
                    evidence,
                });
            }
        }
        Ok(report)
    }
}
