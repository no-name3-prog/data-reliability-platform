//! Built-in validator plugins.

use async_trait::async_trait;
use indexmap::IndexMap;
use regex::Regex;
use serde_json::{json, Value};

use drp_common::{Error, Result};
use drp_core::{
    Asset, CheckDefinition, CheckResult, Plugin, PluginCapability, PluginContext, PluginInfo,
    ValidatorPlugin,
};

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
        let column = required_param_str(check, "column")?;
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
        let column = required_param_str(check, "column")?;
        let mut seen = std::collections::HashSet::new();
        let mut dupes = 0u64;
        for row in rows {
            if let Some(v) = row.get(column) {
                if !v.is_null() && !seen.insert(v.to_string()) {
                    dupes += 1;
                }
            }
        }
        if dupes == 0 {
            Ok(
                CheckResult::passed(check.id, format!("column '{column}' values are unique"))
                    .with_metric("duplicate_count", json!(0)),
            )
        } else {
            Ok(CheckResult::failed(
                check.id,
                check.severity,
                format!("column '{column}' has {dupes} duplicate value(s)"),
            )
            .with_metric("duplicate_count", json!(dupes)))
        }
    }
}

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
        let column = required_param_str(check, "column")?;
        let pattern = required_param_str(check, "pattern")?;
        let re =
            Regex::new(pattern).map_err(|e| Error::validation(format!("invalid regex: {e}")))?;

        let mut mismatches = 0u64;
        for row in rows {
            if let Some(Value::String(s)) = row.get(column) {
                if !re.is_match(s) {
                    mismatches += 1;
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

fn required_param_str<'a>(check: &'a CheckDefinition, key: &str) -> Result<&'a str> {
    check
        .params
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::validation(format!("check param '{key}' is required (string)")))
}
