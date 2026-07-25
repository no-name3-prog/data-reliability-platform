//! Full statistical profiler: null%, unique values, min/max/avg, histograms, semantic types.

use std::collections::HashSet;

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};

use drp_common::{DataType, Result, RunId, UtcTimestamp};
use drp_core::{
    Asset, ColumnProfile, DatasetProfile, Plugin, PluginCapability, PluginContext, PluginInfo,
    ProfilerPlugin, SemanticType,
};

use crate::semantic::{semantic_from_physical, value_as_str};
use crate::stats::{
    as_f64, categorical_histogram, default_bins, insert_numeric_stats, numeric_histogram,
    numeric_summary, top_categories,
};

/// Default column-stats profiler with semantic typing and histograms.
pub struct BasicProfiler {
    info: PluginInfo,
}

impl BasicProfiler {
    /// Create the built-in statistical profiler.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("basic", "Statistical Profiler", env!("CARGO_PKG_VERSION"))
                .with_description(
                    "Row count, null%, unique values, min/max/avg, histograms, semantic types",
                )
                .with_capability(PluginCapability::Profiler),
        }
    }
}

impl Default for BasicProfiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for BasicProfiler {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ProfilerPlugin for BasicProfiler {
    async fn profile(
        &self,
        asset: &Asset,
        rows: &[IndexMap<String, Value>],
        _ctx: &PluginContext,
    ) -> Result<DatasetProfile> {
        let col_names: Vec<(String, DataType)> = if asset.columns.is_empty() {
            rows.first()
                .map(|r| {
                    r.keys()
                        .map(|k| (k.clone(), infer_physical_type(rows, k)))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            asset
                .columns
                .iter()
                .map(|c| (c.name.clone(), c.data_type))
                .collect()
        };

        let row_count = rows.len() as u64;
        let mut column_profiles = Vec::with_capacity(col_names.len());

        for (name, data_type) in col_names {
            let mut null_count = 0u64;
            let mut distinct = HashSet::new();
            let mut nums: Vec<f64> = Vec::new();
            let mut strings: Vec<String> = Vec::new();

            for row in rows {
                match row.get(&name) {
                    None | Some(Value::Null) => null_count += 1,
                    Some(v) => {
                        distinct.insert(canonical_key(v));
                        if let Some(n) = as_f64(v) {
                            nums.push(n);
                        }
                        if let Some(s) = value_as_str(v) {
                            strings.push(s.to_string());
                        }
                    }
                }
            }

            let str_samples: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();

            let non_null = row_count.saturating_sub(null_count);
            let null_percentage = if row_count == 0 {
                0.0
            } else {
                (null_count as f64 / row_count as f64) * 100.0
            };
            let unique_ratio = if non_null == 0 {
                0.0
            } else {
                distinct.len() as f64 / non_null as f64
            };

            let summary = numeric_summary(&nums);
            let (semantic_type, semantic_confidence) =
                semantic_from_physical(data_type, distinct.len() as u64, non_null, &str_samples);

            let histogram = if !nums.is_empty()
                && matches!(
                    data_type,
                    DataType::Integer | DataType::Float | DataType::Unknown
                )
                && semantic_type != SemanticType::Category
            {
                numeric_histogram(&nums, default_bins())
            } else if !strings.is_empty() {
                categorical_histogram(&strings, top_categories())
            } else {
                Vec::new()
            };

            let mut stats = IndexMap::new();
            insert_numeric_stats(&mut stats, &summary);
            stats.insert("null_ratio".into(), json!(null_percentage / 100.0));
            stats.insert("null_percentage".into(), json!(null_percentage));
            stats.insert("unique_ratio".into(), json!(unique_ratio));
            stats.insert("non_null_count".into(), json!(non_null));
            stats.insert(
                "semantic_type".into(),
                json!(format!("{semantic_type:?}").to_ascii_lowercase()),
            );

            let (min, max) = if summary.min.is_some() {
                (summary.min.map(|v| json!(v)), summary.max.map(|v| json!(v)))
            } else if !strings.is_empty() {
                let mut sorted = strings.clone();
                sorted.sort();
                (
                    sorted.first().map(|s| json!(s)),
                    sorted.last().map(|s| json!(s)),
                )
            } else {
                (None, None)
            };

            column_profiles.push(ColumnProfile {
                name,
                data_type,
                semantic_type,
                semantic_confidence,
                null_count,
                null_percentage,
                distinct_count: distinct.len() as u64,
                unique_ratio,
                min,
                max,
                average: summary.mean,
                stddev: summary.stddev,
                histogram,
                stats,
            });
        }

        Ok(DatasetProfile {
            run_id: RunId::new(),
            asset_id: asset.id,
            asset_fqn: Some(asset.fqn.clone()),
            profiler: Some(self.info.id.clone()),
            connector: Some(asset.location.connector.clone()),
            sample_size: Some(row_count),
            row_count,
            columns: column_profiles,
            profiled_at: UtcTimestamp::now(),
        })
    }
}

fn canonical_key(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn infer_physical_type(rows: &[IndexMap<String, Value>], col: &str) -> DataType {
    for row in rows {
        match row.get(col) {
            Some(Value::Bool(_)) => return DataType::Boolean,
            Some(Value::Number(n)) if n.is_i64() || n.is_u64() => return DataType::Integer,
            Some(Value::Number(_)) => return DataType::Float,
            Some(Value::String(_)) => return DataType::String,
            Some(Value::Array(_)) | Some(Value::Object(_)) => return DataType::Complex,
            _ => {}
        }
    }
    DataType::Unknown
}
