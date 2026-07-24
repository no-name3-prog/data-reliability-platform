//! Platform façade — composition handle shared by API and jobs.

use std::sync::Arc;

use drp_common::AppConfig;

use crate::events::EventBus;
use crate::plugin::PluginRegistry;

/// Shared platform state.
#[derive(Clone)]
pub struct Platform {
    /// Loaded configuration.
    pub config: Arc<AppConfig>,
    /// Plugin registry.
    pub plugins: PluginRegistry,
    /// Domain event bus.
    pub events: EventBus,
}

impl Platform {
    /// Build a platform from config with an empty registry.
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(config),
            plugins: PluginRegistry::new(),
            events: EventBus::new(),
        }
    }

    /// Environment label.
    pub fn environment(&self) -> &str {
        &self.config.platform.environment
    }

    /// Whether this is a production environment.
    pub fn is_production(&self) -> bool {
        self.config
            .platform
            .environment
            .eq_ignore_ascii_case("production")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_defaults() {
        let p = Platform::new(AppConfig::default());
        assert!(!p.is_production());
        assert!(p.plugins.is_empty());
    }
}
