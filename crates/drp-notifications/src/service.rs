//! Notification service.

use indexmap::IndexMap;
use serde_json::Value;
use tracing::instrument;

use drp_common::Result;
use drp_core::{PluginContext, PluginRegistry};

/// Fan-out notification service.
#[derive(Clone)]
pub struct NotificationService {
    plugins: PluginRegistry,
    default_channels: Vec<String>,
    enabled: bool,
}

impl NotificationService {
    /// Create a notification service.
    pub fn new(plugins: PluginRegistry, default_channels: Vec<String>, enabled: bool) -> Self {
        Self {
            plugins,
            default_channels,
            enabled,
        }
    }

    /// Send to the default channels.
    #[instrument(skip(self, metadata))]
    pub async fn notify(
        &self,
        subject: &str,
        body: &str,
        metadata: IndexMap<String, Value>,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        self.notify_channels(&self.default_channels, subject, body, metadata)
            .await
    }

    /// Send to explicit channels.
    pub async fn notify_channels(
        &self,
        channels: &[String],
        subject: &str,
        body: &str,
        metadata: IndexMap<String, Value>,
    ) -> Result<()> {
        let ctx = PluginContext::new();
        for channel in channels {
            let plugin = self.plugins.notification(channel)?;
            plugin.send(subject, body, &metadata, &ctx).await?;
        }
        Ok(())
    }
}
