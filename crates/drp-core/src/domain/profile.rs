//! Dataset / column profile model with statistics, histograms, and semantic types.
//!
//! Profiles are versioned by [`DatasetProfile::run_id`] and timestamp so history
//! can be stored and compared over time.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use drp_common::{AssetId, DataType, RunId, UtcTimestamp};

/// Detected semantic meaning of a column beyond its physical type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SemanticType {
    /// No strong semantic signal.
    #[default]
    Unknown,
    /// Email address.
    Email,
    /// Phone number.
    Phone,
    /// Calendar date (no time).
    Date,
    /// Timestamp / datetime.
    DateTime,
    /// URL / URI.
    Url,
    /// UUID / GUID.
    Uuid,
    /// IPv4 / IPv6 address.
    IpAddress,
    /// Free-form integer identifier.
    IntegerId,
    /// Numeric continuous measurement.
    Numeric,
    /// Categorical / low-cardinality string.
    Category,
    /// Free text.
    Text,
    /// Boolean-like.
    Boolean,
}

/// One histogram bucket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistogramBin {
    /// Inclusive lower bound (numeric) or label start (categorical).
    pub label: String,
    /// Count of values in this bin.
    pub count: u64,
    /// Optional lower edge for numeric bins.
    #[serde(default)]
    pub lo: Option<f64>,
    /// Optional upper edge for numeric bins.
    #[serde(default)]
    pub hi: Option<f64>,
}

/// Per-column statistics and semantic classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProfile {
    /// Column name.
    pub name: String,
    /// Physical / inferred data type.
    pub data_type: DataType,
    /// Semantic type (email, phone, date, …).
    #[serde(default)]
    pub semantic_type: SemanticType,
    /// Confidence of semantic classification in `[0, 1]`.
    #[serde(default)]
    pub semantic_confidence: f64,
    /// Null count in the profiled sample (or full scan).
    pub null_count: u64,
    /// Null percentage in `[0, 100]`.
    #[serde(default)]
    pub null_percentage: f64,
    /// Distinct non-null values (exact on sample).
    pub distinct_count: u64,
    /// Distinct ratio among non-null values (`distinct / non_null`).
    #[serde(default)]
    pub unique_ratio: f64,
    /// Minimum (numeric or lexicographic for strings when set).
    #[serde(default)]
    pub min: Option<serde_json::Value>,
    /// Maximum.
    #[serde(default)]
    pub max: Option<serde_json::Value>,
    /// Arithmetic mean for numeric columns.
    #[serde(default)]
    pub average: Option<f64>,
    /// Sample standard deviation for numeric columns.
    #[serde(default)]
    pub stddev: Option<f64>,
    /// Histogram (numeric equal-width or top categorical values).
    #[serde(default)]
    pub histogram: Vec<HistogramBin>,
    /// Extra metrics (extensible).
    #[serde(default)]
    pub stats: IndexMap<String, serde_json::Value>,
}

/// Summary profile for a dataset/asset at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetProfile {
    /// Profile run id (unique history key).
    pub run_id: RunId,
    /// Profiled asset.
    pub asset_id: AssetId,
    /// Fully-qualified name snapshot (optional convenience).
    #[serde(default)]
    pub asset_fqn: Option<String>,
    /// Profiler plugin id used.
    #[serde(default)]
    pub profiler: Option<String>,
    /// Connector id used for sampling (when known).
    #[serde(default)]
    pub connector: Option<String>,
    /// Sample size requested.
    #[serde(default)]
    pub sample_size: Option<u64>,
    /// Row count in the profiled sample (or estimated full count).
    pub row_count: u64,
    /// Column profiles.
    pub columns: Vec<ColumnProfile>,
    /// When profiling completed.
    pub profiled_at: UtcTimestamp,
}

/// Diff between two profiles for the same asset (history comparison).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDiff {
    /// Asset id.
    pub asset_id: AssetId,
    /// Older run id.
    pub baseline_run_id: RunId,
    /// Newer run id.
    pub current_run_id: RunId,
    /// Baseline timestamp.
    pub baseline_at: UtcTimestamp,
    /// Current timestamp.
    pub current_at: UtcTimestamp,
    /// Delta in row count (`current - baseline`).
    pub row_count_delta: i64,
    /// Per-column changes.
    pub columns: Vec<ColumnProfileDiff>,
}

/// Per-column change summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProfileDiff {
    /// Column name.
    pub name: String,
    /// Null percentage delta (points).
    pub null_percentage_delta: Option<f64>,
    /// Distinct count delta.
    pub distinct_count_delta: Option<i64>,
    /// Average delta (numeric).
    pub average_delta: Option<f64>,
    /// Semantic type changed.
    pub semantic_type_changed: bool,
    /// Previous semantic type.
    #[serde(default)]
    pub semantic_type_from: Option<SemanticType>,
    /// Current semantic type.
    #[serde(default)]
    pub semantic_type_to: Option<SemanticType>,
}

impl DatasetProfile {
    /// Compare `self` (current) against a baseline profile.
    pub fn diff_from(&self, baseline: &DatasetProfile) -> ProfileDiff {
        let mut cols = Vec::new();
        for cur in &self.columns {
            let base = baseline.columns.iter().find(|c| c.name == cur.name);
            let (null_delta, distinct_delta, avg_delta, sem_changed, from, to) = match base {
                Some(b) => (
                    Some(cur.null_percentage - b.null_percentage),
                    Some(cur.distinct_count as i64 - b.distinct_count as i64),
                    match (cur.average, b.average) {
                        (Some(a), Some(bb)) => Some(a - bb),
                        _ => None,
                    },
                    cur.semantic_type != b.semantic_type,
                    Some(b.semantic_type),
                    Some(cur.semantic_type),
                ),
                None => (None, None, None, true, None, Some(cur.semantic_type)),
            };
            cols.push(ColumnProfileDiff {
                name: cur.name.clone(),
                null_percentage_delta: null_delta,
                distinct_count_delta: distinct_delta,
                average_delta: avg_delta,
                semantic_type_changed: sem_changed,
                semantic_type_from: from,
                semantic_type_to: to,
            });
        }
        ProfileDiff {
            asset_id: self.asset_id,
            baseline_run_id: baseline.run_id,
            current_run_id: self.run_id,
            baseline_at: baseline.profiled_at,
            current_at: self.profiled_at,
            row_count_delta: self.row_count as i64 - baseline.row_count as i64,
            columns: cols,
        }
    }
}
