//! Shared parameter helpers for validator plugins.
//!
//! New rules should use these helpers so param errors are consistent.

use serde_json::Value;

use drp_common::{Error, Result};
use drp_core::CheckDefinition;

/// Required string param.
pub fn required_str<'a>(check: &'a CheckDefinition, key: &str) -> Result<&'a str> {
    check
        .params
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::validation(format!("check param '{key}' is required (string)")))
}

/// Optional string param.
pub fn optional_str<'a>(check: &'a CheckDefinition, key: &str) -> Option<&'a str> {
    check.params.get(key).and_then(|v| v.as_str())
}

/// Required f64 param (accepts number or numeric string).
pub fn optional_f64(check: &CheckDefinition, key: &str) -> Result<Option<f64>> {
    match check.params.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => Ok(Some(value_as_f64(v).ok_or_else(|| {
            Error::validation(format!("check param '{key}' must be a number"))
        })?)),
    }
}

/// Required u64 param.
pub fn optional_u64(check: &CheckDefinition, key: &str) -> Result<Option<u64>> {
    match check.params.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .or_else(|| n.as_f64().map(|f| f as u64))
            .map(Some)
            .ok_or_else(|| Error::validation(format!("check param '{key}' must be an integer"))),
        Some(Value::String(s)) => s
            .parse::<u64>()
            .map(Some)
            .map_err(|_| Error::validation(format!("check param '{key}' must be an integer"))),
        _ => Err(Error::validation(format!(
            "check param '{key}' must be an integer"
        ))),
    }
}

/// Array of values (any JSON) as string keys for set membership.
pub fn required_value_set(
    check: &CheckDefinition,
    key: &str,
) -> Result<std::collections::HashSet<String>> {
    let arr = check
        .params
        .get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::validation(format!("check param '{key}' is required (array)")))?;
    Ok(arr.iter().map(value_key).collect())
}

/// Canonical string key for a JSON value (for uniqueness / membership).
pub fn value_key(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        other => other.to_string(),
    }
}

/// Coerce JSON value to f64 when possible.
pub fn value_as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}
