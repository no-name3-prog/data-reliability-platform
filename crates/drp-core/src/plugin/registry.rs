//! Typed plugin registry.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::info;

use super::traits::{
    ConnectorPlugin, NotificationPlugin, PluginInfo, ProfilerPlugin, ValidatorPlugin,
};
use drp_common::{Error, Result};

/// Central registry of platform plugins.
#[derive(Clone, Default)]
pub struct PluginRegistry {
    connectors: Arc<RwLock<HashMap<String, Arc<dyn ConnectorPlugin>>>>,
    profilers: Arc<RwLock<HashMap<String, Arc<dyn ProfilerPlugin>>>>,
    validators: Arc<RwLock<HashMap<String, Arc<dyn ValidatorPlugin>>>>,
    notifications: Arc<RwLock<HashMap<String, Arc<dyn NotificationPlugin>>>>,
}

impl PluginRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a connector plugin.
    pub fn register_connector(&self, plugin: Arc<dyn ConnectorPlugin>) {
        let id = plugin.info().id.clone();
        info!(plugin_id = %id, "registering connector plugin");
        self.connectors.write().insert(id, plugin);
    }

    /// Register a profiler plugin.
    pub fn register_profiler(&self, plugin: Arc<dyn ProfilerPlugin>) {
        let id = plugin.info().id.clone();
        info!(plugin_id = %id, "registering profiler plugin");
        self.profilers.write().insert(id, plugin);
    }

    /// Register a validator plugin.
    pub fn register_validator(&self, plugin: Arc<dyn ValidatorPlugin>) {
        let id = plugin.info().id.clone();
        info!(plugin_id = %id, "registering validator plugin");
        self.validators.write().insert(id, plugin);
    }

    /// Register a notification plugin.
    pub fn register_notification(&self, plugin: Arc<dyn NotificationPlugin>) {
        let id = plugin.info().id.clone();
        info!(plugin_id = %id, "registering notification plugin");
        self.notifications.write().insert(id, plugin);
    }

    /// Resolve a connector by id.
    pub fn connector(&self, id: &str) -> Result<Arc<dyn ConnectorPlugin>> {
        self.connectors
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::plugin(format!("connector plugin not found: {id}")))
    }

    /// Resolve a profiler by id.
    pub fn profiler(&self, id: &str) -> Result<Arc<dyn ProfilerPlugin>> {
        self.profilers
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::plugin(format!("profiler plugin not found: {id}")))
    }

    /// Resolve a validator by id.
    pub fn validator(&self, id: &str) -> Result<Arc<dyn ValidatorPlugin>> {
        self.validators
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::plugin(format!("validator plugin not found: {id}")))
    }

    /// Resolve a notification channel by id.
    pub fn notification(&self, id: &str) -> Result<Arc<dyn NotificationPlugin>> {
        self.notifications
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::plugin(format!("notification plugin not found: {id}")))
    }

    /// List all registered plugin metadata.
    pub fn list_all(&self) -> Vec<PluginInfo> {
        let mut out = Vec::new();
        for p in self.connectors.read().values() {
            out.push(p.info().clone());
        }
        for p in self.profilers.read().values() {
            out.push(p.info().clone());
        }
        for p in self.validators.read().values() {
            out.push(p.info().clone());
        }
        for p in self.notifications.read().values() {
            out.push(p.info().clone());
        }
        out
    }

    /// Number of registered plugins across all categories.
    pub fn len(&self) -> usize {
        self.connectors.read().len()
            + self.profilers.read().len()
            + self.validators.read().len()
            + self.notifications.read().len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::traits::{Plugin, PluginCapability, PluginInfo};

    struct DummyConnector {
        info: PluginInfo,
    }

    impl Plugin for DummyConnector {
        fn info(&self) -> &PluginInfo {
            &self.info
        }
    }

    #[async_trait::async_trait]
    impl ConnectorPlugin for DummyConnector {
        async fn test_connection(
            &self,
            _: &drp_common::SourceLocation,
            _: &crate::plugin::PluginContext,
        ) -> Result<()> {
            Ok(())
        }
        async fn discover(
            &self,
            _: &drp_common::SourceLocation,
            _: &crate::plugin::PluginContext,
        ) -> Result<Vec<crate::domain::Asset>> {
            Ok(vec![])
        }
        async fn sample_rows(
            &self,
            _: &crate::domain::Asset,
            _: usize,
            _: &crate::plugin::PluginContext,
        ) -> Result<Vec<indexmap::IndexMap<String, serde_json::Value>>> {
            Ok(vec![])
        }
    }

    #[test]
    fn register_and_resolve_connector() {
        let reg = PluginRegistry::new();
        let plugin = Arc::new(DummyConnector {
            info: PluginInfo::new("dummy", "Dummy", "0.1.0")
                .with_capability(PluginCapability::Connector),
        });
        reg.register_connector(plugin);
        assert!(reg.connector("dummy").is_ok());
        assert!(reg.connector("missing").is_err());
    }
}
