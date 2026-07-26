//! Pluggable OpenAI-compatible chat completions provider.
//!
//! Works with SpaceXAI/xAI (`https://api.x.ai/v1`), Ollama, vLLM, OpenAI, or any
//! server that exposes `POST /chat/completions`.

use async_trait::async_trait;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

use drp_common::{Error, OpenAiCompatibleConfig, Result};
use drp_core::{
    AiMessage, AiProviderPlugin, AiRequest, AiResponse, AiRole, Plugin, PluginCapability,
    PluginContext, PluginInfo,
};

/// HTTP AI provider using the OpenAI chat-completions wire format.
pub struct OpenAiCompatibleProvider {
    info: PluginInfo,
    base_url: String,
    api_key: Option<String>,
    default_model: String,
}

impl OpenAiCompatibleProvider {
    /// Build from config (reads API key from the configured env var).
    pub fn from_config(cfg: &OpenAiCompatibleConfig) -> Self {
        let api_key = if cfg.api_key_env.is_empty() {
            None
        } else {
            std::env::var(&cfg.api_key_env)
                .ok()
                .filter(|s| !s.is_empty())
        };
        Self {
            info: PluginInfo::new(
                "openai_compatible",
                "OpenAI-Compatible LLM",
                env!("CARGO_PKG_VERSION"),
            )
            .with_description(format!(
                "Pluggable OpenAI-compatible provider (base_url={})",
                cfg.base_url
            ))
            .with_capability(PluginCapability::AiProvider),
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            api_key,
            default_model: cfg.model.clone(),
        }
    }

    /// Construct with explicit settings (tests / custom registration).
    pub fn new(
        base_url: impl Into<String>,
        api_key: Option<String>,
        default_model: impl Into<String>,
    ) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        Self {
            info: PluginInfo::new(
                "openai_compatible",
                "OpenAI-Compatible LLM",
                env!("CARGO_PKG_VERSION"),
            )
            .with_description(format!(
                "Pluggable OpenAI-compatible provider (base_url={base_url})"
            ))
            .with_capability(PluginCapability::AiProvider),
            base_url,
            api_key,
            default_model: default_model.into(),
        }
    }
}

impl Plugin for OpenAiCompatibleProvider {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageContent,
}

#[derive(Deserialize)]
struct ChatMessageContent {
    content: Option<String>,
}

#[derive(Deserialize, Default)]
struct ChatUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
}

fn role_str(role: AiRole) -> &'static str {
    match role {
        AiRole::System => "system",
        AiRole::User => "user",
        AiRole::Assistant => "assistant",
    }
}

#[async_trait]
impl AiProviderPlugin for OpenAiCompatibleProvider {
    async fn complete(&self, request: &AiRequest, _ctx: &PluginContext) -> Result<AiResponse> {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());
        let messages: Vec<ChatMessage> = request
            .messages
            .iter()
            .map(|m: &AiMessage| ChatMessage {
                role: role_str(m.role).into(),
                content: m.content.clone(),
            })
            .collect();

        let body = ChatRequest {
            model: model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let url = format!("{}/chat/completions", self.base_url);
        debug!(%url, %model, "openai_compatible complete");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| Error::internal(format!("http client: {e}")))?;

        let mut req = client.post(&url).json(&body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| Error::internal(format!("openai_compatible request failed: {e}")))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| Error::internal(format!("openai_compatible read body: {e}")))?;

        if !status.is_success() {
            return Err(Error::internal(format!(
                "openai_compatible HTTP {status}: {}",
                text.chars().take(500).collect::<String>()
            )));
        }

        let parsed: ChatResponse = serde_json::from_str(&text).map_err(|e| {
            Error::internal(format!(
                "openai_compatible parse response: {e}; body={}",
                text.chars().take(300).collect::<String>()
            ))
        })?;

        let content = parsed
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let mut usage = IndexMap::new();
        if let Some(u) = parsed.usage {
            usage.insert("prompt_tokens".into(), u.prompt_tokens);
            usage.insert("completion_tokens".into(), u.completion_tokens);
            usage.insert("total_tokens".into(), u.total_tokens);
        }

        Ok(AiResponse {
            provider: self.info.id.clone(),
            content,
            model: parsed.model.or(Some(model)),
            usage,
            metadata: IndexMap::new(),
        })
    }

    async fn health(&self, _ctx: &PluginContext) -> Result<()> {
        if self.api_key.is_none() && self.base_url.contains("api.x.ai") {
            return Err(Error::config(
                "openai_compatible: no API key found in configured env var (e.g. XAI_API_KEY)",
            ));
        }
        Ok(())
    }
}
