//! Offline heuristic AI provider that suggests validation rules from schema + profiling.
//!
//! No network calls. Used as the default pluggable provider and as a fallback when
//! remote LLM responses cannot be parsed.

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};

use drp_common::{DataType, Result, Severity};
use drp_core::{
    AiProviderPlugin, AiRequest, AiResponse, AiRole, Asset, ColumnProfile, DatasetProfile, Plugin,
    PluginCapability, PluginContext, PluginInfo, ProposedRule, SemanticType,
};

/// Deterministic offline AI provider for rule suggestions and stub completions.
#[derive(Clone)]
pub struct HeuristicAiProvider {
    info: PluginInfo,
}

impl HeuristicAiProvider {
    /// Create the heuristic provider.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new(
                "heuristic",
                "Heuristic Rule Suggester",
                env!("CARGO_PKG_VERSION"),
            )
            .with_description(
                "Offline AI layer that proposes validation rules from schema, profiles, and samples",
            )
            .with_capability(PluginCapability::AiProvider),
        }
    }

    /// Suggest validation rules from asset schema, optional profile, and sample rows.
    pub fn suggest_rules(
        &self,
        asset: &Asset,
        profile: Option<&DatasetProfile>,
        sample_rows: &[IndexMap<String, Value>],
        available_validators: &[String],
    ) -> Vec<(ProposedRule, String, f64)> {
        let has = |id: &str| available_validators.iter().any(|v| v == id);
        let mut out: Vec<(ProposedRule, String, f64)> = Vec::new();

        // Row count: if we saw any rows, suggest a minimum of 1.
        if has("row_count")
            && (profile
                .map(|p| p.row_count > 0)
                .unwrap_or(!sample_rows.is_empty()))
        {
            let min = 1u64;
            let rule = ProposedRule::new(format!("{} has rows", asset.name), "row_count")
                .with_description("Ensure the dataset is not empty")
                .with_param("min", json!(min))
                .with_severity(Severity::Error);
            out.push((
                rule,
                "Profile/sample shows non-zero row count; guard against empty loads.".into(),
                0.85,
            ));
        }

        let columns: Vec<(String, Option<&ColumnProfile>)> = if let Some(p) = profile {
            p.columns
                .iter()
                .map(|c| (c.name.clone(), Some(c)))
                .collect()
        } else {
            asset
                .columns
                .iter()
                .map(|c| (c.name.clone(), None))
                .collect()
        };

        for (col_name, col_prof) in columns {
            let lower = col_name.to_lowercase();
            let schema_col = asset.columns.iter().find(|c| c.name == col_name);
            let nullable = schema_col.map(|c| c.nullable).unwrap_or(true);
            let data_type = col_prof
                .map(|c| c.data_type)
                .or_else(|| schema_col.map(|c| c.data_type))
                .unwrap_or(DataType::Unknown);
            let semantic = col_prof
                .map(|c| c.semantic_type)
                .unwrap_or(SemanticType::Unknown);
            let null_pct = col_prof.map(|c| c.null_percentage).unwrap_or(0.0);
            let unique_ratio = col_prof.map(|c| c.unique_ratio).unwrap_or(0.0);
            let distinct = col_prof.map(|c| c.distinct_count).unwrap_or(0);

            // not_null for required / low-null columns
            if has("not_null") {
                let looks_key = lower == "id"
                    || lower.ends_with("_id")
                    || lower == "email"
                    || lower.ends_with("_email")
                    || lower.contains("uuid");
                let low_null = null_pct < 1.0;
                if (!nullable && low_null) || (looks_key && null_pct < 5.0) || null_pct == 0.0 {
                    let conf = if !nullable || null_pct == 0.0 {
                        0.95
                    } else {
                        0.75
                    };
                    let rule = ProposedRule::new(format!("{col_name} is not null"), "not_null")
                        .with_description(format!("Column '{col_name}' should not contain nulls"))
                        .with_param("column", json!(col_name))
                        .with_severity(Severity::Error);
                    out.push((
                        rule,
                        format!(
                            "Column '{col_name}' has null_percentage={null_pct:.1}% (nullable={nullable})."
                        ),
                        conf,
                    ));
                }
            }

            // unique for high-cardinality ids
            if has("unique") {
                let looks_id = lower == "id"
                    || lower.ends_with("_id")
                    || lower.contains("uuid")
                    || semantic == SemanticType::Uuid
                    || semantic == SemanticType::IntegerId;
                if looks_id && unique_ratio >= 0.98 && distinct > 1 {
                    let rule = ProposedRule::new(format!("{col_name} is unique"), "unique")
                        .with_description(format!("Column '{col_name}' appears to be a unique key"))
                        .with_param("column", json!(col_name))
                        .with_severity(Severity::Error);
                    out.push((
                        rule,
                        format!(
                            "Column '{col_name}' unique_ratio={unique_ratio:.3}, distinct={distinct}."
                        ),
                        0.8,
                    ));
                }
            }

            // regex for email / uuid semantics
            if has("regex") {
                if semantic == SemanticType::Email
                    || lower.contains("email")
                    || lower.ends_with("_email")
                {
                    let rule =
                        ProposedRule::new(format!("{col_name} matches email format"), "regex")
                            .with_description("Values should look like email addresses")
                            .with_param("column", json!(col_name))
                            .with_param("pattern", json!(r"^[^@\s]+@[^@\s]+\.[^@\s]+$"))
                            .with_severity(Severity::Warning);
                    out.push((
                        rule,
                        format!("Column '{col_name}' classified as email (semantic={semantic:?})."),
                        0.7,
                    ));
                }
                if semantic == SemanticType::Uuid || lower.contains("uuid") {
                    let rule = ProposedRule::new(
                        format!("{col_name} matches UUID format"),
                        "regex",
                    )
                    .with_description("Values should be UUID-shaped")
                    .with_param("column", json!(col_name))
                    .with_param(
                        "pattern",
                        json!(
                            r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
                        ),
                    )
                    .with_severity(Severity::Warning);
                    out.push((
                        rule,
                        format!("Column '{col_name}' classified as UUID."),
                        0.72,
                    ));
                }
            }

            // range for numeric columns with min/max
            if has("range")
                && matches!(data_type, DataType::Integer | DataType::Float)
                && col_prof.is_some()
            {
                if let Some(cp) = col_prof {
                    let min_v = cp.min.as_ref().and_then(|v| v.as_f64());
                    let max_v = cp.max.as_ref().and_then(|v| v.as_f64());
                    if let (Some(min), Some(max)) = (min_v, max_v) {
                        // slight padding
                        let pad = ((max - min).abs() * 0.05).max(1.0);
                        let lo = min - pad;
                        let hi = max + pad;
                        // skip absurd unbounded ranges
                        if hi.is_finite() && lo.is_finite() && hi >= lo {
                            let rule = ProposedRule::new(
                                format!("{col_name} within observed range"),
                                "range",
                            )
                            .with_description(format!(
                                "Numeric column '{col_name}' should stay near observed bounds"
                            ))
                            .with_param("column", json!(col_name))
                            .with_param("min", json!(lo))
                            .with_param("max", json!(hi))
                            .with_severity(Severity::Warning);
                            out.push((
                                rule,
                                format!(
                                    "Observed min={min}, max={max} for '{col_name}'; suggesting padded range."
                                ),
                                0.55,
                            ));
                        }
                    }
                }
            }

            // accepted_values for low-cardinality categories
            if has("accepted_values")
                && (semantic == SemanticType::Category
                    || (distinct > 0 && distinct <= 12 && unique_ratio < 0.5))
            {
                let mut values: Vec<Value> = Vec::new();
                if let Some(cp) = col_prof {
                    for bin in cp.histogram.iter().take(12) {
                        if !bin.label.is_empty() && bin.label != "(null)" {
                            values.push(json!(bin.label));
                        }
                    }
                }
                // also sample rows
                if values.is_empty() {
                    let mut seen = std::collections::BTreeSet::new();
                    for row in sample_rows.iter().take(200) {
                        if let Some(v) = row.get(&col_name) {
                            if !v.is_null() {
                                let s = match v {
                                    Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                };
                                seen.insert(s);
                            }
                        }
                        if seen.len() >= 12 {
                            break;
                        }
                    }
                    values = seen.into_iter().map(|s| json!(s)).collect();
                }
                if values.len() >= 2 && values.len() <= 12 {
                    let rule = ProposedRule::new(
                        format!("{col_name} in accepted values"),
                        "accepted_values",
                    )
                    .with_description(format!(
                        "Low-cardinality column '{col_name}' should stay within known labels"
                    ))
                    .with_param("column", json!(col_name))
                    .with_param("values", json!(values))
                    .with_severity(Severity::Warning);
                    out.push((
                        rule,
                        format!("Column '{col_name}' looks categorical (distinct={distinct})."),
                        0.6,
                    ));
                }
            }

            // freshness for timestamp columns
            if has("freshness")
                && (matches!(data_type, DataType::Timestamp | DataType::Date)
                    || matches!(semantic, SemanticType::DateTime | SemanticType::Date)
                    || lower.contains("updated")
                    || lower.ends_with("_at")
                    || lower.contains("timestamp"))
            {
                let rule = ProposedRule::new(format!("{col_name} is fresh"), "freshness")
                    .with_description(format!(
                        "Timestamp column '{col_name}' should not lag more than 7 days"
                    ))
                    .with_param("column", json!(col_name))
                    .with_param("max_age_secs", json!(604_800u64))
                    .with_severity(Severity::Warning);
                out.push((
                    rule,
                    format!("Column '{col_name}' appears temporal; suggesting 7-day freshness."),
                    0.5,
                ));
            }
        }

        // Cap volume so review stays manageable
        out.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        out.truncate(25);
        out
    }
}

impl Default for HeuristicAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for HeuristicAiProvider {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl AiProviderPlugin for HeuristicAiProvider {
    async fn complete(&self, request: &AiRequest, _ctx: &PluginContext) -> Result<AiResponse> {
        // If the user message looks like a rule-suggestion JSON context, return
        // structured suggestions; otherwise a short summary.
        let last_user = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == AiRole::User)
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let content = if last_user.contains("\"asset\"") || last_user.contains("suggest") {
            // Callers that use complete() for suggestions should prefer the
            // dedicated suggest path; still return valid empty JSON array.
            r#"{"suggestions":[]}"#.to_string()
        } else {
            format!(
                "[heuristic] Offline AI provider. Provide schema/profile context via the rule-suggestion API. Last message length: {}.",
                last_user.len()
            )
        };

        Ok(AiResponse {
            provider: self.info.id.clone(),
            content,
            model: Some("heuristic-1".into()),
            usage: IndexMap::new(),
            metadata: IndexMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drp_common::{AssetKind, SourceLocation};
    use drp_core::ColumnMeta;

    #[test]
    fn suggests_not_null_for_id() {
        let p = HeuristicAiProvider::new();
        let mut asset = Asset::new(
            "demo.public.customers",
            "customers",
            AssetKind::Table,
            SourceLocation::new("mock", "mock://customers"),
        );
        asset.columns = vec![
            ColumnMeta::new("id", DataType::Integer).required(),
            ColumnMeta::new("email", DataType::String),
        ];
        let validators = vec![
            "not_null".into(),
            "unique".into(),
            "regex".into(),
            "row_count".into(),
        ];
        let suggestions = p.suggest_rules(&asset, None, &[], &validators);
        assert!(
            suggestions
                .iter()
                .any(|s| s.0.validator == "not_null" && s.0.params.get("column").is_some()),
            "expected not_null suggestion: {suggestions:?}"
        );
    }
}
