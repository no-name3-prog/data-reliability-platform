//! Built-in validator plugins (data quality rules).
//!
//! | Plugin id | Purpose | Key params |
//! |-----------|---------|------------|
//! | `not_null` | Column has no nulls | `column` |
//! | `unique` | Column values are unique | `column` |
//! | `accepted_values` | Values ⊆ allowed set | `column`, `values` |
//! | `regex` | Values match pattern | `column`, `pattern` |
//! | `range` | Numeric min/max | `column`, `min?`, `max?` |
//! | `freshness` | Max age of timestamps | `column?`, `max_age_secs` |
//! | `row_count` | Row count bounds | `min?`, `max?` |
//! | `referential_integrity` | FK-like membership | `column`, `values` or context `reference_values` |

use std::collections::HashSet;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use indexmap::IndexMap;
use regex::Regex;
use serde_json::{json, Value};

use drp_common::{Error, Result};
use drp_core::{
    Asset, CheckDefinition, CheckResult, Plugin, PluginCapability, PluginContext, PluginInfo,
    ValidatorPlugin,
};

use crate::params::{
    optional_f64, optional_str, optional_u64, required_str, required_value_set, value_as_f64,
    value_key,
};

// ---------------------------------------------------------------------------
// not_null
// ---------------------------------------------------------------------------

/// Assert a column has no null values.
pub struct NotNullValidator {
    info: PluginInfo,
}

impl NotNullValidator {
    /// Create the not-null validator.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("not_null", "Not Null", env!("CARGO_PKG_VERSION"))
                .with_description("Fails when the target column contains nulls")
                .with_capability(PluginCapability::Validator),
        }
    }
}

impl Default for NotNullValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for NotNullValidator {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ValidatorPlugin for NotNullValidator {
    async fn validate(
        &self,
        check: &CheckDefinition,
        _asset: &Asset,
        rows: &[IndexMap<String, Value>],
        _ctx: &PluginContext,
    ) -> Result<CheckResult> {
        let column = required_str(check, "column")?;
        let mut nulls = 0u64;
        for row in rows {
            match row.get(column) {
                None | Some(Value::Null) => nulls += 1,
                _ => {}
            }
        }
        if nulls == 0 {
            Ok(
                CheckResult::passed(check.id, format!("column '{column}' has no nulls"))
                    .with_metric("null_count", json!(0))
                    .with_metric("row_count", json!(rows.len())),
            )
        } else {
            Ok(CheckResult::failed(
                check.id,
                check.severity,
                format!("column '{column}' has {nulls} null value(s)"),
            )
            .with_metric("null_count", json!(nulls))
            .with_metric("row_count", json!(rows.len())))
        }
    }
}

// ---------------------------------------------------------------------------
// unique
// ---------------------------------------------------------------------------

/// Assert a column has unique values.
pub struct UniqueValidator {
    info: PluginInfo,
}

impl UniqueValidator {
    /// Create the unique validator.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("unique", "Unique", env!("CARGO_PKG_VERSION"))
                .with_description("Fails when the target column has duplicate values")
                .with_capability(PluginCapability::Validator),
        }
    }
}

impl Default for UniqueValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for UniqueValidator {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ValidatorPlugin for UniqueValidator {
    async fn validate(
        &self,
        check: &CheckDefinition,
        _asset: &Asset,
        rows: &[IndexMap<String, Value>],
        _ctx: &PluginContext,
    ) -> Result<CheckResult> {
        let column = required_str(check, "column")?;
        let mut seen = HashSet::new();
        let mut dupes = 0u64;
        for row in rows {
            if let Some(v) = row.get(column) {
                if !v.is_null() && !seen.insert(value_key(v)) {
                    dupes += 1;
                }
            }
        }
        if dupes == 0 {
            Ok(
                CheckResult::passed(check.id, format!("column '{column}' values are unique"))
                    .with_metric("duplicate_count", json!(0))
                    .with_metric("distinct_count", json!(seen.len())),
            )
        } else {
            Ok(CheckResult::failed(
                check.id,
                check.severity,
                format!("column '{column}' has {dupes} duplicate value(s)"),
            )
            .with_metric("duplicate_count", json!(dupes))
            .with_metric("distinct_count", json!(seen.len())))
        }
    }
}

// ---------------------------------------------------------------------------
// accepted_values
// ---------------------------------------------------------------------------

/// Assert every non-null value is in an allowed set.
pub struct AcceptedValuesValidator {
    info: PluginInfo,
}

impl AcceptedValuesValidator {
    /// Create the accepted-values validator.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new(
                "accepted_values",
                "Accepted Values",
                env!("CARGO_PKG_VERSION"),
            )
            .with_description("Fails when values are outside the allowed set")
            .with_capability(PluginCapability::Validator),
        }
    }
}

impl Default for AcceptedValuesValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for AcceptedValuesValidator {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ValidatorPlugin for AcceptedValuesValidator {
    async fn validate(
        &self,
        check: &CheckDefinition,
        _asset: &Asset,
        rows: &[IndexMap<String, Value>],
        _ctx: &PluginContext,
    ) -> Result<CheckResult> {
        let column = required_str(check, "column")?;
        let allowed = required_value_set(check, "values")?;
        let mut invalid = 0u64;
        for row in rows {
            match row.get(column) {
                None | Some(Value::Null) => {}
                Some(v) => {
                    if !allowed.contains(&value_key(v)) {
                        invalid += 1;
                    }
                }
            }
        }
        if invalid == 0 {
            Ok(CheckResult::passed(
                check.id,
                format!("column '{column}' values are all accepted"),
            )
            .with_metric("invalid_count", json!(0))
            .with_metric("allowed_count", json!(allowed.len())))
        } else {
            Ok(CheckResult::failed(
                check.id,
                check.severity,
                format!("column '{column}' has {invalid} value(s) outside accepted set"),
            )
            .with_metric("invalid_count", json!(invalid))
            .with_metric("allowed_count", json!(allowed.len())))
        }
    }
}

// ---------------------------------------------------------------------------
// regex
// ---------------------------------------------------------------------------

/// Assert a column matches a regex.
pub struct RegexValidator {
    info: PluginInfo,
}

impl RegexValidator {
    /// Create the regex validator.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("regex", "Regex Match", env!("CARGO_PKG_VERSION"))
                .with_description("Fails when values do not match the given pattern")
                .with_capability(PluginCapability::Validator),
        }
    }
}

impl Default for RegexValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RegexValidator {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ValidatorPlugin for RegexValidator {
    async fn validate(
        &self,
        check: &CheckDefinition,
        _asset: &Asset,
        rows: &[IndexMap<String, Value>],
        _ctx: &PluginContext,
    ) -> Result<CheckResult> {
        let column = required_str(check, "column")?;
        let pattern = required_str(check, "pattern")?;
        let re =
            Regex::new(pattern).map_err(|e| Error::validation(format!("invalid regex: {e}")))?;

        let mut mismatches = 0u64;
        for row in rows {
            if let Some(Value::String(s)) = row.get(column) {
                if !re.is_match(s) {
                    mismatches += 1;
                }
            } else if let Some(v) = row.get(column) {
                if !v.is_null() {
                    let s = value_key(v);
                    if !re.is_match(&s) {
                        mismatches += 1;
                    }
                }
            }
        }

        if mismatches == 0 {
            Ok(
                CheckResult::passed(check.id, format!("column '{column}' matches /{pattern}/"))
                    .with_metric("mismatch_count", json!(0)),
            )
        } else {
            Ok(CheckResult::failed(
                check.id,
                check.severity,
                format!("column '{column}' has {mismatches} value(s) not matching /{pattern}/"),
            )
            .with_metric("mismatch_count", json!(mismatches)))
        }
    }
}

// ---------------------------------------------------------------------------
// range
// ---------------------------------------------------------------------------

/// Assert numeric values fall within [min, max] (bounds optional).
pub struct RangeValidator {
    info: PluginInfo,
}

impl RangeValidator {
    /// Create the range validator.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("range", "Range", env!("CARGO_PKG_VERSION"))
                .with_description("Fails when numeric values fall outside min/max")
                .with_capability(PluginCapability::Validator),
        }
    }
}

impl Default for RangeValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RangeValidator {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ValidatorPlugin for RangeValidator {
    async fn validate(
        &self,
        check: &CheckDefinition,
        _asset: &Asset,
        rows: &[IndexMap<String, Value>],
        _ctx: &PluginContext,
    ) -> Result<CheckResult> {
        let column = required_str(check, "column")?;
        let min = optional_f64(check, "min")?;
        let max = optional_f64(check, "max")?;
        if min.is_none() && max.is_none() {
            return Err(Error::validation(
                "range requires at least one of 'min' or 'max'",
            ));
        }

        let mut out_of_range = 0u64;
        let mut non_numeric = 0u64;
        for row in rows {
            match row.get(column) {
                None | Some(Value::Null) => {}
                Some(v) => match value_as_f64(v) {
                    Some(n) => {
                        if min.map(|m| n < m).unwrap_or(false)
                            || max.map(|m| n > m).unwrap_or(false)
                        {
                            out_of_range += 1;
                        }
                    }
                    None => non_numeric += 1,
                },
            }
        }

        let bad = out_of_range + non_numeric;
        if bad == 0 {
            Ok(CheckResult::passed(
                check.id,
                format!("column '{column}' values are within range"),
            )
            .with_metric("out_of_range", json!(0))
            .with_metric("min", json!(min))
            .with_metric("max", json!(max)))
        } else {
            Ok(CheckResult::failed(
                check.id,
                check.severity,
                format!(
                    "column '{column}' has {out_of_range} out-of-range and {non_numeric} non-numeric value(s)"
                ),
            )
            .with_metric("out_of_range", json!(out_of_range))
            .with_metric("non_numeric", json!(non_numeric))
            .with_metric("min", json!(min))
            .with_metric("max", json!(max)))
        }
    }
}

// ---------------------------------------------------------------------------
// freshness
// ---------------------------------------------------------------------------

/// Assert the newest timestamp in a column (or asset updated_at) is recent enough.
pub struct FreshnessValidator {
    info: PluginInfo,
}

impl FreshnessValidator {
    /// Create the freshness validator.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("freshness", "Freshness", env!("CARGO_PKG_VERSION"))
                .with_description("Fails when data is older than max_age_secs")
                .with_capability(PluginCapability::Validator),
        }
    }
}

impl Default for FreshnessValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for FreshnessValidator {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ValidatorPlugin for FreshnessValidator {
    async fn validate(
        &self,
        check: &CheckDefinition,
        asset: &Asset,
        rows: &[IndexMap<String, Value>],
        _ctx: &PluginContext,
    ) -> Result<CheckResult> {
        let max_age_secs = optional_u64(check, "max_age_secs")?.ok_or_else(|| {
            Error::validation("freshness requires param 'max_age_secs' (integer seconds)")
        })?;
        let column = optional_str(check, "column");

        let newest = if let Some(col) = column {
            let mut best: Option<DateTime<Utc>> = None;
            for row in rows {
                if let Some(v) = row.get(col) {
                    if let Some(ts) = parse_timestamp(v) {
                        best = Some(best.map_or(ts, |b| b.max(ts)));
                    }
                }
            }
            best
        } else {
            Some(asset.updated_at.inner())
        };

        let Some(newest) = newest else {
            return Ok(CheckResult::failed(
                check.id,
                check.severity,
                "freshness: no parseable timestamps found",
            )
            .with_metric("max_age_secs", json!(max_age_secs)));
        };

        let age_secs = (Utc::now() - newest).num_seconds().max(0) as u64;
        if age_secs <= max_age_secs {
            Ok(CheckResult::passed(
                check.id,
                format!("data is fresh (age {age_secs}s <= {max_age_secs}s)"),
            )
            .with_metric("age_secs", json!(age_secs))
            .with_metric("max_age_secs", json!(max_age_secs))
            .with_metric("newest", json!(newest.to_rfc3339())))
        } else {
            Ok(CheckResult::failed(
                check.id,
                check.severity,
                format!("data is stale (age {age_secs}s > {max_age_secs}s)"),
            )
            .with_metric("age_secs", json!(age_secs))
            .with_metric("max_age_secs", json!(max_age_secs))
            .with_metric("newest", json!(newest.to_rfc3339())))
        }
    }
}

fn parse_timestamp(v: &Value) -> Option<DateTime<Utc>> {
    match v {
        Value::String(s) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Some(dt.with_timezone(&Utc));
            }
            if let Ok(dt) = DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S %z") {
                return Some(dt.with_timezone(&Utc));
            }
            if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                return Some(ndt.and_utc());
            }
            if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                return Some(ndt.and_utc());
            }
            if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return Some(d.and_hms_opt(0, 0, 0)?.and_utc());
            }
            None
        }
        Value::Number(n) => {
            // Unix seconds or millis
            let f = n.as_f64()?;
            let secs = if f > 1e12 { f / 1000.0 } else { f };
            DateTime::from_timestamp(secs as i64, 0)
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// row_count
// ---------------------------------------------------------------------------

/// Assert sampled row count is within bounds.
pub struct RowCountValidator {
    info: PluginInfo,
}

impl RowCountValidator {
    /// Create the row-count validator.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("row_count", "Row Count", env!("CARGO_PKG_VERSION"))
                .with_description("Fails when sampled row count is outside min/max")
                .with_capability(PluginCapability::Validator),
        }
    }
}

impl Default for RowCountValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RowCountValidator {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ValidatorPlugin for RowCountValidator {
    async fn validate(
        &self,
        check: &CheckDefinition,
        _asset: &Asset,
        rows: &[IndexMap<String, Value>],
        _ctx: &PluginContext,
    ) -> Result<CheckResult> {
        let min = optional_u64(check, "min")?;
        let max = optional_u64(check, "max")?;
        if min.is_none() && max.is_none() {
            return Err(Error::validation(
                "row_count requires at least one of 'min' or 'max'",
            ));
        }
        let n = rows.len() as u64;
        let below = min.map(|m| n < m).unwrap_or(false);
        let above = max.map(|m| n > m).unwrap_or(false);
        if !below && !above {
            Ok(
                CheckResult::passed(check.id, format!("row count {n} is within bounds"))
                    .with_metric("row_count", json!(n))
                    .with_metric("min", json!(min))
                    .with_metric("max", json!(max)),
            )
        } else {
            Ok(CheckResult::failed(
                check.id,
                check.severity,
                format!("row count {n} outside bounds min={min:?} max={max:?}"),
            )
            .with_metric("row_count", json!(n))
            .with_metric("min", json!(min))
            .with_metric("max", json!(max)))
        }
    }
}

// ---------------------------------------------------------------------------
// referential_integrity
// ---------------------------------------------------------------------------

/// Assert column values exist in a reference set (FK-like).
///
/// Reference values come from:
/// 1. Check param `values` (array), or
/// 2. Plugin context config key `reference_values` (array) — typically injected
///    by [`crate::ValidationService`] when `reference_asset_id` + `reference_column`
///    are provided on the check.
pub struct ReferentialIntegrityValidator {
    info: PluginInfo,
}

impl ReferentialIntegrityValidator {
    /// Create the referential integrity validator.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new(
                "referential_integrity",
                "Referential Integrity",
                env!("CARGO_PKG_VERSION"),
            )
            .with_description("Fails when values are missing from a reference set")
            .with_capability(PluginCapability::Validator),
        }
    }
}

impl Default for ReferentialIntegrityValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ReferentialIntegrityValidator {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ValidatorPlugin for ReferentialIntegrityValidator {
    async fn validate(
        &self,
        check: &CheckDefinition,
        _asset: &Asset,
        rows: &[IndexMap<String, Value>],
        ctx: &PluginContext,
    ) -> Result<CheckResult> {
        let column = required_str(check, "column")?;
        let reference: HashSet<String> =
            if let Some(arr) = check.params.get("values").and_then(|v| v.as_array()) {
                arr.iter().map(value_key).collect()
            } else if let Some(arr) = ctx
                .config
                .get("reference_values")
                .and_then(|v| v.as_array())
            {
                arr.iter().map(value_key).collect()
            } else {
                return Err(Error::validation(
                    "referential_integrity requires param 'values' or context 'reference_values' \
                 (from reference_asset_id + reference_column)",
                ));
            };

        let mut missing = 0u64;
        for row in rows {
            match row.get(column) {
                None | Some(Value::Null) => {}
                Some(v) => {
                    if !reference.contains(&value_key(v)) {
                        missing += 1;
                    }
                }
            }
        }

        if missing == 0 {
            Ok(CheckResult::passed(
                check.id,
                format!("column '{column}' satisfies referential integrity"),
            )
            .with_metric("missing_count", json!(0))
            .with_metric("reference_size", json!(reference.len())))
        } else {
            Ok(CheckResult::failed(
                check.id,
                check.severity,
                format!("column '{column}' has {missing} value(s) missing from reference set"),
            )
            .with_metric("missing_count", json!(missing))
            .with_metric("reference_size", json!(reference.len())))
        }
    }
}
