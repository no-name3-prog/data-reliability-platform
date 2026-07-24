//! Built-in AI provider registration.

use std::sync::Arc;

use drp_core::PluginRegistry;

use crate::EchoAiProvider;

/// Register built-in AI providers.
pub fn register_builtin_ai_providers(registry: &PluginRegistry) {
    registry.register_ai_provider(Arc::new(EchoAiProvider::new()));
}
