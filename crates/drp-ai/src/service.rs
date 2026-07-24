//! AI service — resolves providers by id.

use tracing::instrument;

use drp_common::Result;
use drp_core::{AiRequest, AiResponse, PluginContext, PluginRegistry};

/// Thin façade over [`AiProviderPlugin`](drp_core::AiProviderPlugin) instances.
#[derive(Clone)]
pub struct AiService {
    plugins: PluginRegistry,
    default_provider: String,
}

impl AiService {
    /// Create an AI service.
    pub fn new(plugins: PluginRegistry) -> Self {
        Self {
            plugins,
            default_provider: "echo".into(),
        }
    }

    /// Complete a request with the given (or default) provider.
    #[instrument(skip(self, request))]
    pub async fn complete(
        &self,
        request: AiRequest,
        provider_id: Option<&str>,
    ) -> Result<AiResponse> {
        let id = provider_id.unwrap_or(&self.default_provider);
        let provider = self.plugins.ai_provider(id)?;
        provider.complete(&request, &PluginContext::new()).await
    }
}
