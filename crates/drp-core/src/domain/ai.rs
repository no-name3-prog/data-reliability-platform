//! AI / LLM provider domain types (engines implement [`crate::AiProviderPlugin`]).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

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
