//! Offline echo AI provider for tests and local demos.

use async_trait::async_trait;
use indexmap::IndexMap;

use drp_common::Result;
use drp_core::{
    AiMessage, AiProviderPlugin, AiRequest, AiResponse, AiRole, Plugin, PluginCapability,
    PluginContext, PluginInfo,
};

/// Returns a deterministic stub completion (no network).
pub struct EchoAiProvider {
    info: PluginInfo,
}

impl EchoAiProvider {
    /// Create the echo provider.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("echo", "Echo AI Provider", env!("CARGO_PKG_VERSION"))
                .with_description("Offline stub that echoes the last user message")
                .with_capability(PluginCapability::AiProvider),
        }
    }
}

impl Default for EchoAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for EchoAiProvider {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl AiProviderPlugin for EchoAiProvider {
    async fn complete(&self, request: &AiRequest, _ctx: &PluginContext) -> Result<AiResponse> {
        let last_user = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == AiRole::User)
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let content = format!(
            "[echo] I received your request ({} message(s)). Last user text: {}",
            request.messages.len(),
            last_user
        );

        let mut usage = IndexMap::new();
        usage.insert("prompt_tokens".into(), last_user.len() as u64);
        usage.insert("completion_tokens".into(), content.len() as u64);

        Ok(AiResponse {
            provider: self.info.id.clone(),
            content,
            model: request.model.clone().or_else(|| Some("echo-1".into())),
            usage,
            metadata: IndexMap::new(),
        })
    }
}

/// Helper to build a simple single-user request.
pub fn simple_user_request(text: impl Into<String>) -> AiRequest {
    AiRequest {
        model: None,
        messages: vec![AiMessage::user(text)],
        temperature: None,
        max_tokens: None,
        options: IndexMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drp_core::PluginContext;

    #[tokio::test]
    async fn echo_returns_content() {
        let p = EchoAiProvider::new();
        let resp = p
            .complete(&simple_user_request("hello"), &PluginContext::new())
            .await
            .unwrap();
        assert!(resp.content.contains("hello"));
        assert_eq!(resp.provider, "echo");
    }
}
