//! AI / LLM provider domain types (engines implement [`crate::AiProviderPlugin`]).
//!
//! Also hosts **validation rule suggestions**: AI-proposed checks that stay
//! inactive until a human reviews and approves them.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use drp_common::{AssetId, CheckId, RunId, Severity, SuggestionId, UtcTimestamp};

use super::CheckDefinition;

/// Role of a chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiRole {
    /// System instruction.
    System,
    /// User turn.
    User,
    /// Assistant turn.
    Assistant,
}

/// One message in an AI conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    /// Role.
    pub role: AiRole,
    /// Content text.
    pub content: String,
}

impl AiMessage {
    /// System message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::System,
            content: content.into(),
        }
    }

    /// User message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::User,
            content: content.into(),
        }
    }
}

/// Request to an AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequest {
    /// Logical model name (provider-specific).
    #[serde(default)]
    pub model: Option<String>,
    /// Conversation / prompt messages.
    pub messages: Vec<AiMessage>,
    /// Temperature when supported.
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Max tokens when supported.
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Provider-specific options.
    #[serde(default)]
    pub options: IndexMap<String, serde_json::Value>,
}

/// Response from an AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    /// Provider plugin id.
    pub provider: String,
    /// Primary text content.
    pub content: String,
    /// Optional model id actually used.
    #[serde(default)]
    pub model: Option<String>,
    /// Token usage if known.
    #[serde(default)]
    pub usage: IndexMap<String, u64>,
    /// Raw metadata.
    #[serde(default)]
    pub metadata: IndexMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Rule suggestions (human-in-the-loop)
// ---------------------------------------------------------------------------

/// Lifecycle of an AI-suggested validation rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSuggestionStatus {
    /// Awaiting human review — **not** active as a check.
    #[default]
    Pending,
    /// User approved; a [`CheckDefinition`] was created and is active.
    Approved,
    /// User rejected; no check was created.
    Rejected,
}

/// Proposed validation rule payload (mirrors a check, without ids / schedule).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedRule {
    /// Suggested human-readable name.
    pub name: String,
    /// Optional description / rationale short form.
    #[serde(default)]
    pub description: Option<String>,
    /// Validator plugin id (must match a registered rule, e.g. `not_null`).
    pub validator: String,
    /// Severity when the check fails.
    #[serde(default)]
    pub severity: Severity,
    /// Plugin-specific parameters.
    #[serde(default)]
    pub params: IndexMap<String, serde_json::Value>,
}

impl ProposedRule {
    /// Build a pending-ready proposed rule.
    pub fn new(name: impl Into<String>, validator: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            validator: validator.into(),
            severity: Severity::Error,
            params: IndexMap::new(),
        }
    }

    /// Attach a parameter.
    pub fn with_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    /// Set severity.
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Materialize an enabled check definition for the given asset.
    pub fn into_check(&self, asset_id: AssetId) -> CheckDefinition {
        let mut check = CheckDefinition::new(self.name.clone(), asset_id, self.validator.clone())
            .with_severity(self.severity);
        check.description = self.description.clone();
        check.params = self.params.clone();
        check.enabled = true;
        check
    }
}

/// AI-suggested validation rule waiting for human review.
///
/// Suggestions are **never** active until status is [`RuleSuggestionStatus::Approved`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSuggestion {
    /// Unique suggestion id.
    pub id: SuggestionId,
    /// Target asset.
    pub asset_id: AssetId,
    /// Review status.
    pub status: RuleSuggestionStatus,
    /// The proposed rule (becomes a check on approve).
    pub proposed: ProposedRule,
    /// Longer rationale from the AI / heuristic engine.
    #[serde(default)]
    pub rationale: String,
    /// Confidence score in `[0, 1]` when available.
    #[serde(default)]
    pub confidence: f64,
    /// AI provider plugin id that produced the suggestion.
    pub provider: String,
    /// Optional model id used.
    #[serde(default)]
    pub model: Option<String>,
    /// Profile run that informed the suggestion (when available).
    #[serde(default)]
    pub profile_run_id: Option<RunId>,
    /// Connector used to sample rows.
    #[serde(default)]
    pub connector_id: Option<String>,
    /// Check created after approval (if any).
    #[serde(default)]
    pub approved_check_id: Option<CheckId>,
    /// Optional rejection reason.
    #[serde(default)]
    pub rejection_reason: Option<String>,
    /// Who reviewed (actor string).
    #[serde(default)]
    pub reviewed_by: Option<String>,
    /// When the suggestion was created.
    pub created_at: UtcTimestamp,
    /// When it was approved or rejected.
    #[serde(default)]
    pub reviewed_at: Option<UtcTimestamp>,
}

impl RuleSuggestion {
    /// Create a new pending suggestion.
    pub fn pending(
        asset_id: AssetId,
        proposed: ProposedRule,
        provider: impl Into<String>,
        rationale: impl Into<String>,
        confidence: f64,
    ) -> Self {
        Self {
            id: SuggestionId::new(),
            asset_id,
            status: RuleSuggestionStatus::Pending,
            proposed,
            rationale: rationale.into(),
            confidence: confidence.clamp(0.0, 1.0),
            provider: provider.into(),
            model: None,
            profile_run_id: None,
            connector_id: None,
            approved_check_id: None,
            rejection_reason: None,
            reviewed_by: None,
            created_at: UtcTimestamp::now(),
            reviewed_at: None,
        }
    }

    /// Attach profile / connector context.
    pub fn with_context(
        mut self,
        profile_run_id: Option<RunId>,
        connector_id: Option<String>,
        model: Option<String>,
    ) -> Self {
        self.profile_run_id = profile_run_id;
        self.connector_id = connector_id;
        self.model = model;
        self
    }
}
