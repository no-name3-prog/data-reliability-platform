//! Typed plugin registry.
//!
//! Feature services resolve plugins **by id** so new implementations never
//! require changes to orchestration code — only a `register_*` call at
//! composition time.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::info;

use super::traits::{
    AiProviderPlugin, AnomalyDetectorPlugin, ConnectorPlugin, NotificationPlugin, PluginCapability,
    PluginInfo, ProfilerPlugin, ValidatorPlugin,
};
use drp_common::{Error, Result};

/// Central registry of platform plugins.
#[derive(Clone, Default)]
pub struct PluginRegistry {
    connectors: Arc<RwLock<HashMap<String, Arc<dyn ConnectorPlugin>>>>,
    profilers: Arc<RwLock<HashMap<String, Arc<dyn ProfilerPlugin>>>>,
    validators: Arc<RwLock<HashMap<String, Arc<dyn ValidatorPlugin>>>>,
    anomaly_detectors: Arc<RwLock<HashMap<String, Arc<dyn AnomalyDetectorPlugin>>>>,
    notifications: Arc<RwLock<HashMap<String, Arc<dyn NotificationPlugin>>>>,
    ai_providers: Arc<RwLock<HashMap<String, Arc<dyn AiProviderPlugin>>>>,
}

impl PluginRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a connector plugin.
    pub fn register_connector(&self, plugin: Arc<dyn ConnectorPlugin>) {
        let id = plugin.info().id.clone();
        info!(plugin_id = %id, capability = "connector", "registering plugin");
        self.connectors.write().insert(id, plugin);
    }

    /// Register a profiler plugin.
    pub fn register_profiler(&self, plugin: Arc<dyn ProfilerPlugin>) {
        let id = plugin.info().id.clone();
        info!(plugin_id = %id, capability = "profiler", "registering plugin");
        self.profilers.write().insert(id, plugin);
    }

    /// Register a validator / rule plugin.
    pub fn register_validator(&self, plugin: Arc<dyn ValidatorPlugin>) {
        let id = plugin.info().id.clone();
        info!(plugin_id = %id, capability = "validator", "registering plugin");
        self.validators.write().insert(id, plugin);
    }

    /// Register an anomaly detector plugin.
    pub fn register_anomaly_detector(&self, plugin: Arc<dyn AnomalyDetectorPlugin>) {
        let id = plugin.info().id.clone();
        info!(plugin_id = %id, capability = "anomaly_detector", "registering plugin");
        self.anomaly_detectors.write().insert(id, plugin);
    }

    /// Register a notification channel plugin.
    pub fn register_notification(&self, plugin: Arc<dyn NotificationPlugin>) {
        let id = plugin.info().id.clone();
        info!(plugin_id = %id, capability = "notification", "registering plugin");
        self.notifications.write().insert(id, plugin);
    }

    /// Register an AI provider plugin.
    pub fn register_ai_provider(&self, plugin: Arc<dyn AiProviderPlugin>) {
        let id = plugin.info().id.clone();
        info!(plugin_id = %id, capability = "ai_provider", "registering plugin");
        self.ai_providers.write().insert(id, plugin);
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

    /// Resolve an anomaly detector by id.
    pub fn anomaly_detector(&self, id: &str) -> Result<Arc<dyn AnomalyDetectorPlugin>> {
        self.anomaly_detectors
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::plugin(format!("anomaly detector plugin not found: {id}")))
    }

    /// Resolve a notification channel by id.
    pub fn notification(&self, id: &str) -> Result<Arc<dyn NotificationPlugin>> {
        self.notifications
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::plugin(format!("notification plugin not found: {id}")))
    }

    /// Resolve an AI provider by id.
    pub fn ai_provider(&self, id: &str) -> Result<Arc<dyn AiProviderPlugin>> {
        self.ai_providers
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::plugin(format!("AI provider plugin not found: {id}")))
    }

    /// List plugin ids for a capability.
    pub fn ids_for(&self, capability: PluginCapability) -> Vec<String> {
        match capability {
            PluginCapability::Connector => self.connectors.read().keys().cloned().collect(),
            PluginCapability::Profiler => self.profilers.read().keys().cloned().collect(),
            PluginCapability::Validator => self.validators.read().keys().cloned().collect(),
            PluginCapability::AnomalyDetector => {
                self.anomaly_detectors.read().keys().cloned().collect()
            }
            PluginCapability::Notification => self.notifications.read().keys().cloned().collect(),
            PluginCapability::AiProvider => self.ai_providers.read().keys().cloned().collect(),
            PluginCapability::Extension => Vec::new(),
        }
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
        for p in self.anomaly_detectors.read().values() {
            out.push(p.info().clone());
        }
        for p in self.notifications.read().values() {
            out.push(p.info().clone());
        }
        for p in self.ai_providers.read().values() {
            out.push(p.info().clone());
        }
        out
    }

    /// Number of registered plugins across all categories.
    pub fn len(&self) -> usize {
        self.connectors.read().len()
            + self.profilers.read().len()
            + self.validators.read().len()
            + self.anomaly_detectors.read().len()
            + self.notifications.read().len()
            + self.ai_providers.read().len()
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
        assert!(reg
            .ids_for(PluginCapability::Connector)
            .contains(&"dummy".into()));
    }
}
