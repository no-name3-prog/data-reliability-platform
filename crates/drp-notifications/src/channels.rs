//! Built-in notification channel plugins.

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::Value;
use tracing::info;

use drp_common::Result;
use drp_core::{NotificationPlugin, Plugin, PluginCapability, PluginContext, PluginInfo};

/// Logs notifications via `tracing`.
pub struct LogNotifier {
    info: PluginInfo,
}

impl LogNotifier {
    /// Create a log notifier.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("log", "Log Notifier", env!("CARGO_PKG_VERSION"))
                .with_description("Writes alerts to the application log")
                .with_capability(PluginCapability::Notification),
        }
    }
}

impl Default for LogNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for LogNotifier {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl NotificationPlugin for LogNotifier {
    async fn send(
        &self,
        subject: &str,
        body: &str,
        metadata: &IndexMap<String, Value>,
        _ctx: &PluginContext,
    ) -> Result<()> {
        info!(%subject, %body, ?metadata, "notification");
        Ok(())
    }
}
