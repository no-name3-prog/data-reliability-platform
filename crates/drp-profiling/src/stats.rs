//! Statistical helpers: histograms, numeric summary.

use indexmap::IndexMap;
use serde_json::{json, Value};

use drp_core::HistogramBin;

const DEFAULT_BINS: usize = 10;
const TOP_CATEGORIES: usize = 10;

/// Numeric summary.
#[derive(Debug, Default)]
pub struct NumericSummary {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    pub stddev: Option<f64>,
}

/// Compute min/max/mean/stddev for a set of numbers.
pub fn numeric_summary(values: &[f64]) -> NumericSummary {
    if values.is_empty() {
        return NumericSummary::default();
    }
    let mut min = values[0];
    let mut max = values[0];
    let mut sum = 0.0;
    for &v in values {
        min = min.min(v);
        max = max.max(v);
        sum += v;
    }
    let n = values.len() as f64;
    let mean = sum / n;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    NumericSummary {
        min: Some(min),
        max: Some(max),
        mean: Some(mean),
        stddev: Some(var.sqrt()),
    }
}

/// Equal-width histogram for numeric values.
pub fn numeric_histogram(values: &[f64], bins: usize) -> Vec<HistogramBin> {
    if values.is_empty() {
        return Vec::new();
    }
    let bins = bins.max(1);
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if (max - min).abs() < f64::EPSILON {
        return vec![HistogramBin {
            label: format!("{min:.4}"),
            count: values.len() as u64,
            lo: Some(min),
            hi: Some(max),
        }];
    }
    let width = (max - min) / bins as f64;
    let mut counts = vec![0u64; bins];
    for &v in values {
        let mut idx = ((v - min) / width).floor() as usize;
        if idx >= bins {
            idx = bins - 1;
        }
        counts[idx] += 1;
    }
    counts
        .into_iter()
        .enumerate()
        .map(|(i, count)| {
            let lo = min + i as f64 * width;
            let hi = lo + width;
            HistogramBin {
                label: format!("[{lo:.4}, {hi:.4})"),
                count,
                lo: Some(lo),
                hi: Some(hi),
            }
        })
        .collect()
}

/// Top-N categorical histogram by frequency.
pub fn categorical_histogram(values: &[String], top_n: usize) -> Vec<HistogramBin> {
    let mut freq: IndexMap<String, u64> = IndexMap::new();
    for v in values {
        *freq.entry(v.clone()).or_insert(0) += 1;
    }
    let mut items: Vec<_> = freq.into_iter().collect();
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    items.truncate(top_n.max(1));
    items
        .into_iter()
        .map(|(label, count)| HistogramBin {
            label,
            count,
            lo: None,
            hi: None,
        })
        .collect()
}

/// Parse JSON value as f64 when possible.
pub fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Default bin count for numeric histograms.
pub fn default_bins() -> usize {
    DEFAULT_BINS
}

/// Default top-N for categorical histograms.
pub fn top_categories() -> usize {
    TOP_CATEGORIES
}

/// Serialize numeric edges into stats map helpers.
pub fn insert_numeric_stats(stats: &mut IndexMap<String, Value>, summary: &NumericSummary) {
    if let Some(min) = summary.min {
        stats.insert("min".into(), json!(min));
    }
    if let Some(max) = summary.max {
        stats.insert("max".into(), json!(max));
    }
    if let Some(mean) = summary.mean {
        stats.insert("mean".into(), json!(mean));
        stats.insert("average".into(), json!(mean));
    }
    if let Some(sd) = summary.stddev {
        stats.insert("stddev".into(), json!(sd));
    }
}
