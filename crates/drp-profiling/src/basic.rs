//! Basic statistical profiler plugin.

use std::collections::HashSet;

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};

use drp_common::{DataType, Result, RunId, UtcTimestamp};
use drp_core::{
    Asset, ColumnProfile, DatasetProfile, Plugin, PluginCapability, PluginContext, PluginInfo,
    ProfilerPlugin,
};

/// Default column-stats profiler.
pub struct BasicProfiler {
    info: PluginInfo,
}

impl BasicProfiler {
    /// Create the built-in basic profiler.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("basic", "Basic Profiler", env!("CARGO_PKG_VERSION"))
                .with_description("Null counts, distinct counts, and simple numeric stats")
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
                .map(|r| r.keys().map(|k| (k.clone(), infer_type(rows, k))).collect())
                .unwrap_or_default()
        } else {
            asset
                .columns
                .iter()
                .map(|c| (c.name.clone(), c.data_type))
                .collect()
        };

        let mut column_profiles = Vec::with_capacity(col_names.len());
        for (name, data_type) in col_names {
            let mut null_count = 0u64;
            let mut distinct = HashSet::new();
            let mut min_num: Option<f64> = None;
            let mut max_num: Option<f64> = None;
            let mut sum = 0.0f64;
            let mut numeric_n = 0u64;

            for row in rows {
                match row.get(&name) {
                    None | Some(Value::Null) => null_count += 1,
                    Some(v) => {
                        distinct.insert(v.to_string());
                        if let Some(n) = as_f64(v) {
                            min_num = Some(min_num.map_or(n, |m| m.min(n)));
                            max_num = Some(max_num.map_or(n, |m| m.max(n)));
                            sum += n;
                            numeric_n += 1;
                        }
                    }
                }
            }

            let mut stats = IndexMap::new();
            if let Some(min) = min_num {
                stats.insert("min".into(), json!(min));
            }
            if let Some(max) = max_num {
                stats.insert("max".into(), json!(max));
            }
            if numeric_n > 0 {
                stats.insert("mean".into(), json!(sum / numeric_n as f64));
            }
            let null_ratio = if rows.is_empty() {
                0.0
            } else {
                null_count as f64 / rows.len() as f64
            };
            stats.insert("null_ratio".into(), json!(null_ratio));

            column_profiles.push(ColumnProfile {
                name,
                data_type,
                null_count,
                distinct_count: distinct.len() as u64,
                stats,
            });
        }

        Ok(DatasetProfile {
            run_id: RunId::new(),
            asset_id: asset.id,
            row_count: rows.len() as u64,
            columns: column_profiles,
            profiled_at: UtcTimestamp::now(),
        })
    }
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn infer_type(rows: &[IndexMap<String, Value>], col: &str) -> DataType {
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
