//! Rule suggestion service: propose validation rules from schema + profile + samples.
//!
//! Suggestions stay **pending** until a human approves (creates an active check)
//! or rejects them.

use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{info, instrument, warn};

use drp_common::{AiConfig, AssetId, Error, Result, Severity, SuggestionId, UtcTimestamp};
use drp_core::{
    AiMessage, AiRequest, Asset, CheckDefinition, DatasetProfile, PluginContext, PluginInfo,
    PluginRegistry, ProposedRule, RuleSuggestion, RuleSuggestionStatus,
};
use drp_storage::Store;

use crate::heuristic::HeuristicAiProvider;

/// Orchestrates AI-assisted validation rule suggestions with human review.
#[derive(Clone)]
pub struct RuleSuggestionService {
    store: Arc<dyn Store>,
    plugins: PluginRegistry,
    config: AiConfig,
    heuristic: HeuristicAiProvider,
}

impl RuleSuggestionService {
    /// Create the suggestion service.
    pub fn new(store: Arc<dyn Store>, plugins: PluginRegistry, config: AiConfig) -> Self {
        Self {
            store,
            plugins,
            config,
            heuristic: HeuristicAiProvider::new(),
        }
    }

    /// Whether the optional AI layer is enabled.
    pub fn enabled(&self) -> bool {
        self.config.enabled
    }

    /// Default provider id.
    pub fn default_provider(&self) -> &str {
        &self.config.default_provider
    }

    /// List registered AI providers.
    pub fn list_providers(&self) -> Vec<PluginInfo> {
        self.plugins
            .list_all()
            .into_iter()
            .filter(|p| {
                p.capabilities
                    .iter()
                    .any(|c| matches!(c, drp_core::PluginCapability::AiProvider))
            })
            .collect()
    }

    /// Generate suggestions for an asset (status = pending). Does **not** activate checks.
    #[instrument(skip(self), fields(asset_id = %asset_id))]
    pub async fn suggest_for_asset(
        &self,
        asset_id: &AssetId,
        connector_id: &str,
        provider_id: Option<&str>,
    ) -> Result<Vec<RuleSuggestion>> {
        if !self.config.enabled {
            return Err(Error::config(
                "AI layer is disabled (set ai.enabled=true to use rule suggestions)",
            ));
        }

        let asset = self
            .store
            .get_asset(asset_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("asset {asset_id}")))?;

        let connector = self.plugins.connector(connector_id)?;
        let ctx = PluginContext::new();
        let sample_limit = self.config.sample_rows.max(1);
        let sample_rows = connector
            .sample_rows(&asset, sample_limit, &ctx)
            .await
            .unwrap_or_else(|e| {
                warn!(error = %e, "sample_rows failed for suggestions; continuing without samples");
                Vec::new()
            });

        let profile = self.store.latest_profile(asset_id).await?;
        let profile_run_id = profile.as_ref().map(|p| p.run_id);

        let available: Vec<String> = self.plugins.ids_for(drp_core::PluginCapability::Validator);

        let provider_id = provider_id.unwrap_or(self.default_provider()).to_string();

        let (proposals, model) = self
            .generate_proposals(
                &provider_id,
                &asset,
                profile.as_ref(),
                &sample_rows,
                &available,
            )
            .await?;

        let mut saved = Vec::with_capacity(proposals.len());
        for (proposed, rationale, confidence) in proposals {
            // Skip unknown validators
            if !available.iter().any(|v| v == &proposed.validator) {
                warn!(
                    validator = %proposed.validator,
                    "skipping suggestion for unknown validator"
                );
                continue;
            }
            let suggestion = RuleSuggestion::pending(
                *asset_id,
                proposed,
                provider_id.clone(),
                rationale,
                confidence,
            )
            .with_context(
                profile_run_id,
                Some(connector_id.to_string()),
                model.clone(),
            );
            let suggestion = self.store.upsert_rule_suggestion(suggestion).await?;
            saved.push(suggestion);
        }

        info!(
            asset_id = %asset_id,
            provider = %provider_id,
            count = saved.len(),
            "AI rule suggestions created (pending review)"
        );
        Ok(saved)
    }

    async fn generate_proposals(
        &self,
        provider_id: &str,
        asset: &Asset,
        profile: Option<&DatasetProfile>,
        sample_rows: &[IndexMap<String, Value>],
        available: &[String],
    ) -> Result<(Vec<(ProposedRule, String, f64)>, Option<String>)> {
        // Always prefer the direct heuristic path for the heuristic provider.
        if provider_id == "heuristic" {
            return Ok((
                self.heuristic
                    .suggest_rules(asset, profile, sample_rows, available),
                Some("heuristic-1".into()),
            ));
        }

        // Remote / other providers: ask for structured JSON, fall back to heuristic.
        match self
            .complete_structured_suggestions(provider_id, asset, profile, sample_rows, available)
            .await
        {
            Ok((list, model)) if !list.is_empty() => Ok((list, model)),
            Ok((_, model)) => {
                warn!("AI provider returned no parseable suggestions; using heuristic fallback");
                Ok((
                    self.heuristic
                        .suggest_rules(asset, profile, sample_rows, available),
                    model.or_else(|| Some("heuristic-fallback".into())),
                ))
            }
            Err(e) => {
                warn!(error = %e, "AI provider failed; using heuristic fallback");
                Ok((
                    self.heuristic
                        .suggest_rules(asset, profile, sample_rows, available),
                    Some("heuristic-fallback".into()),
                ))
            }
        }
    }

    async fn complete_structured_suggestions(
        &self,
        provider_id: &str,
        asset: &Asset,
        profile: Option<&DatasetProfile>,
        sample_rows: &[IndexMap<String, Value>],
        available: &[String],
    ) -> Result<(Vec<(ProposedRule, String, f64)>, Option<String>)> {
        let provider = self.plugins.ai_provider(provider_id)?;
        let context = build_suggestion_context(asset, profile, sample_rows, available);
        let request = AiRequest {
            model: None,
            messages: vec![
                AiMessage::system(
                    "You are a data quality assistant. Suggest validation rules as JSON only. \
                     Respond with {\"suggestions\":[{\"name\":\"...\",\"validator\":\"...\",\
                     \"severity\":\"error|warning|info|critical\",\"params\":{...},\
                     \"description\":\"...\",\"rationale\":\"...\",\"confidence\":0.0-1.0}]}. \
                     Only use the listed validator ids. Do not invent validators.",
                ),
                AiMessage::user(context),
            ],
            temperature: Some(0.2),
            max_tokens: Some(2048),
            options: IndexMap::new(),
        };

        let resp = provider.complete(&request, &PluginContext::new()).await?;
        let parsed = parse_suggestion_json(&resp.content, available)?;
        Ok((parsed, resp.model))
    }

    /// List suggestions.
    pub async fn list(
        &self,
        asset_id: Option<&AssetId>,
        status: Option<RuleSuggestionStatus>,
        limit: Option<usize>,
    ) -> Result<Vec<RuleSuggestion>> {
        self.store
            .list_rule_suggestions(asset_id, status, limit)
            .await
    }

    /// Get one suggestion.
    pub async fn get(&self, id: &SuggestionId) -> Result<RuleSuggestion> {
        self.store
            .get_rule_suggestion(id)
            .await?
            .ok_or_else(|| Error::not_found(format!("rule suggestion {id}")))
    }

    /// Approve a pending suggestion → creates an enabled [`CheckDefinition`].
    #[instrument(skip(self), fields(suggestion_id = %id))]
    pub async fn approve(
        &self,
        id: &SuggestionId,
        reviewed_by: Option<String>,
    ) -> Result<ApproveResult> {
        let mut suggestion = self.get(id).await?;
        if suggestion.status != RuleSuggestionStatus::Pending {
            return Err(Error::validation(format!(
                "suggestion {id} is {:?} (only pending can be approved)",
                suggestion.status
            )));
        }

        // Ensure validator still exists
        let _ = self.plugins.validator(&suggestion.proposed.validator)?;

        let check: CheckDefinition = suggestion.proposed.into_check(suggestion.asset_id);
        let check = self.store.upsert_check(check).await?;

        suggestion.status = RuleSuggestionStatus::Approved;
        suggestion.approved_check_id = Some(check.id);
        suggestion.reviewed_by = reviewed_by;
        suggestion.reviewed_at = Some(UtcTimestamp::now());
        let suggestion = self.store.upsert_rule_suggestion(suggestion).await?;

        info!(
            suggestion_id = %id,
            check_id = %check.id,
            "AI rule suggestion approved; check is now active"
        );

        Ok(ApproveResult { suggestion, check })
    }

    /// Reject a pending suggestion (no check created).
    #[instrument(skip(self), fields(suggestion_id = %id))]
    pub async fn reject(
        &self,
        id: &SuggestionId,
        reason: Option<String>,
        reviewed_by: Option<String>,
    ) -> Result<RuleSuggestion> {
        let mut suggestion = self.get(id).await?;
        if suggestion.status != RuleSuggestionStatus::Pending {
            return Err(Error::validation(format!(
                "suggestion {id} is {:?} (only pending can be rejected)",
                suggestion.status
            )));
        }
        suggestion.status = RuleSuggestionStatus::Rejected;
        suggestion.rejection_reason = reason;
        suggestion.reviewed_by = reviewed_by;
        suggestion.reviewed_at = Some(UtcTimestamp::now());
        let suggestion = self.store.upsert_rule_suggestion(suggestion).await?;
        info!(suggestion_id = %id, "AI rule suggestion rejected");
        Ok(suggestion)
    }
}

/// Result of approving a suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveResult {
    /// Updated suggestion (status = approved).
    pub suggestion: RuleSuggestion,
    /// Active check definition that was created.
    pub check: CheckDefinition,
}

fn build_suggestion_context(
    asset: &Asset,
    profile: Option<&DatasetProfile>,
    sample_rows: &[IndexMap<String, Value>],
    available: &[String],
) -> String {
    let schema: Vec<Value> = asset
        .columns
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "data_type": c.data_type,
                "nullable": c.nullable,
            })
        })
        .collect();

    let profile_json = profile.map(|p| {
        json!({
            "row_count": p.row_count,
            "run_id": p.run_id.to_string(),
            "columns": p.columns.iter().map(|c| json!({
                "name": c.name,
                "data_type": c.data_type,
                "semantic_type": c.semantic_type,
                "null_percentage": c.null_percentage,
                "distinct_count": c.distinct_count,
                "unique_ratio": c.unique_ratio,
                "min": c.min,
                "max": c.max,
            })).collect::<Vec<_>>(),
        })
    });

    let samples: Vec<Value> = sample_rows
        .iter()
        .take(10)
        .map(|r| {
            let mut map = serde_json::Map::new();
            for (k, v) in r {
                map.insert(k.clone(), v.clone());
            }
            Value::Object(map)
        })
        .collect();

    json!({
        "task": "suggest_validation_rules",
        "asset": {
            "fqn": asset.fqn,
            "name": asset.name,
            "kind": asset.kind,
            "columns": schema,
        },
        "profile": profile_json,
        "sample_rows": samples,
        "available_validators": available,
    })
    .to_string()
}

#[derive(Debug, Deserialize)]
struct LlmSuggestionEnvelope {
    #[serde(default)]
    suggestions: Vec<LlmSuggestionItem>,
}

#[derive(Debug, Deserialize)]
struct LlmSuggestionItem {
    name: String,
    validator: String,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    params: IndexMap<String, Value>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    rationale: Option<String>,
    #[serde(default)]
    confidence: Option<f64>,
}

fn parse_suggestion_json(
    content: &str,
    available: &[String],
) -> Result<Vec<(ProposedRule, String, f64)>> {
    let json_str = extract_json_object(content).ok_or_else(|| {
        Error::validation("AI response did not contain a JSON object with suggestions")
    })?;
    let env: LlmSuggestionEnvelope = serde_json::from_str(json_str)
        .map_err(|e| Error::validation(format!("parse AI suggestions JSON: {e}")))?;

    let mut out = Vec::new();
    for item in env.suggestions {
        if !available.iter().any(|v| v == &item.validator) {
            continue;
        }
        let severity = parse_severity(item.severity.as_deref());
        let mut proposed = ProposedRule::new(item.name, item.validator).with_severity(severity);
        proposed.params = item.params;
        if let Some(d) = item.description {
            proposed.description = Some(d);
        }
        let rationale = item
            .rationale
            .unwrap_or_else(|| "Suggested by AI provider".into());
        let confidence = item.confidence.unwrap_or(0.6).clamp(0.0, 1.0);
        out.push((proposed, rationale, confidence));
    }
    Ok(out)
}

fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end > start {
        Some(&s[start..=end])
    } else {
        None
    }
}

fn parse_severity(s: Option<&str>) -> Severity {
    match s.map(|x| x.to_ascii_lowercase()).as_deref() {
        Some("info") => Severity::Info,
        Some("warning") | Some("warn") => Severity::Warning,
        Some("critical") => Severity::Critical,
        _ => Severity::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_llm_json() {
        let raw = r#"Here you go:
        {"suggestions":[{"name":"id not null","validator":"not_null","params":{"column":"id"},"confidence":0.9}]}
        "#;
        let avail = vec!["not_null".into()];
        let parsed = parse_suggestion_json(raw, &avail).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0.validator, "not_null");
    }
}
