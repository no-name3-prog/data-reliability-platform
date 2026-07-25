//! Simple z-score outlier detector for numeric columns.

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};

use drp_common::Result;
use drp_core::{
    AnomalyDetectorPlugin, AnomalyFinding, AnomalyKind, AnomalyReport, AnomalySeverity, Asset,
    Plugin, PluginCapability, PluginContext, PluginInfo,
};

/// Flags numeric values more than `k` standard deviations from the mean.
pub struct ZScoreDetector {
    info: PluginInfo,
    default_k: f64,
}

impl ZScoreDetector {
    /// Create with default `k = 3.0`.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("zscore", "Z-Score Detector", env!("CARGO_PKG_VERSION"))
                .with_description("Numeric outlier detection via z-score")
                .with_capability(PluginCapability::AnomalyDetector),
            default_k: 3.0,
        }
    }
}

impl Default for ZScoreDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ZScoreDetector {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl AnomalyDetectorPlugin for ZScoreDetector {
    async fn detect(
        &self,
        asset: &Asset,
        rows: &[IndexMap<String, Value>],
        ctx: &PluginContext,
    ) -> Result<AnomalyReport> {
        let k = ctx
            .config
            .get("k")
            .and_then(|v| v.as_f64())
            .unwrap_or(self.default_k);

        let mut report = AnomalyReport::healthy(asset.id, self.info.id.clone());
        if rows.len() < 3 {
            return Ok(report);
        }

        let cols: Vec<String> = rows
            .first()
            .map(|r| r.keys().cloned().collect())
            .unwrap_or_default();

        for col in cols {
            let mut nums: Vec<f64> = Vec::new();
            for row in rows {
                if let Some(n) = row.get(&col).and_then(as_f64) {
                    nums.push(n);
                }
            }
            if nums.len() < 3 {
                continue;
            }
            let mean = nums.iter().sum::<f64>() / nums.len() as f64;
            let var = nums.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / nums.len() as f64;
            let std = var.sqrt();
            if std == 0.0 {
                continue;
            }
            let mut outliers = 0u64;
            let mut max_abs_z = 0.0f64;
            for n in &nums {
                let z = ((n - mean) / std).abs();
                max_abs_z = max_abs_z.max(z);
                if z > k {
                    outliers += 1;
                }
            }
            if outliers > 0 {
                let mut evidence = IndexMap::new();
                evidence.insert("outlier_count".into(), json!(outliers));
                evidence.insert("mean".into(), json!(mean));
                evidence.insert("std".into(), json!(std));
                evidence.insert("k".into(), json!(k));
                evidence.insert("max_abs_z".into(), json!(max_abs_z));
                report.findings.push(AnomalyFinding {
                    detector: self.info.id.clone(),
                    kind: AnomalyKind::Other,
                    field: Some(col.clone()),
                    message: format!(
                        "column '{col}' has {outliers} value(s) beyond {k}σ (max |z|={max_abs_z:.2})"
                    ),
                    severity: AnomalySeverity::Medium,
                    score: Some((max_abs_z / (k * 2.0)).min(1.0)),
                    evidence,
                });
            }
        }
        Ok(report)
    }
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}
