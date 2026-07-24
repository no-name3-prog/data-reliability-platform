//! Dataset / column profile summary.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use drp_common::{AssetId, DataType, RunId, UtcTimestamp};

/// Summary profile for a dataset/asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetProfile {
    /// Profile run id.
    pub run_id: RunId,
    /// Profiled asset.
    pub asset_id: AssetId,
    /// Approximate row count.
    pub row_count: u64,
    /// Column profiles.
    pub columns: Vec<ColumnProfile>,
    /// When profiling completed.
    pub profiled_at: UtcTimestamp,
}

/// Per-column statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProfile {
    /// Column name.
    pub name: String,
    /// Inferred type.
    pub data_type: DataType,
    /// Null count.
    pub null_count: u64,
    /// Distinct count.
    pub distinct_count: u64,
    /// Extra metrics.
    #[serde(default)]
    pub stats: IndexMap<String, serde_json::Value>,
}
