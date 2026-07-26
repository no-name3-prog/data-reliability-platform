//! Built-in AI provider registration.

use std::sync::Arc;

use drp_common::AiConfig;
use drp_core::PluginRegistry;
use tracing::info;

use crate::heuristic::HeuristicAiProvider;
use crate::openai_compatible::OpenAiCompatibleProvider;
use crate::EchoAiProvider;

/// Register built-in AI providers (echo + heuristic).
///
/// For OpenAI-compatible remote/local models, use
/// [`register_ai_providers_with_config`].
pub fn register_builtin_ai_providers(registry: &PluginRegistry) {
    registry.register_ai_provider(Arc::new(EchoAiProvider::new()));
    registry.register_ai_provider(Arc::new(HeuristicAiProvider::new()));
}

/// Register built-in providers plus optional OpenAI-compatible plugin from config.
pub fn register_ai_providers_with_config(registry: &PluginRegistry, config: &AiConfig) {
    register_builtin_ai_providers(registry);
    if config.openai_compatible.enabled {
        info!(
            base_url = %config.openai_compatible.base_url,
            model = %config.openai_compatible.model,
            "registering openai_compatible AI provider"
        );
        registry.register_ai_provider(Arc::new(OpenAiCompatibleProvider::from_config(
            &config.openai_compatible,
        )));
    }
}
