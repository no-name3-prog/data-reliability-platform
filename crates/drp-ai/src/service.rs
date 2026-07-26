//! AI service — resolves providers by id.

use tracing::instrument;

use drp_common::{AiConfig, Result};
use drp_core::{AiRequest, AiResponse, PluginContext, PluginInfo, PluginRegistry};

/// Thin façade over [`AiProviderPlugin`](drp_core::AiProviderPlugin) instances.
#[derive(Clone)]
pub struct AiService {
    plugins: PluginRegistry,
    default_provider: String,
}

impl AiService {
    /// Create an AI service with default provider `echo`.
    pub fn new(plugins: PluginRegistry) -> Self {
        Self {
            plugins,
            default_provider: "echo".into(),
        }
    }

    /// Create from platform AI config (default provider from config).
    pub fn with_config(plugins: PluginRegistry, config: &AiConfig) -> Self {
        Self {
            plugins,
            default_provider: config.default_provider.clone(),
        }
    }

    /// Default provider id.
    pub fn default_provider(&self) -> &str {
        &self.default_provider
    }

    /// List registered AI provider plugins.
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
