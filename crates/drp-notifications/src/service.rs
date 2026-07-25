//! Notification service.

use indexmap::IndexMap;
use serde_json::Value;
use tracing::{instrument, warn};

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

    /// Configured default channel ids.
    pub fn default_channels(&self) -> &[String] {
        &self.default_channels
    }

    /// Send to the default channels (best-effort per channel).
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
        self.notify_channels(&self.default_channels.clone(), subject, body, metadata)
            .await
    }

    /// Send to explicit channels (continues on individual channel errors).
    pub async fn notify_channels(
        &self,
        channels: &[String],
        subject: &str,
        body: &str,
        metadata: IndexMap<String, Value>,
    ) -> Result<()> {
        let ctx = PluginContext::new();
        for channel in channels {
            match self.plugins.notification(channel) {
                Ok(plugin) => {
                    if let Err(e) = plugin.send(subject, body, &metadata, &ctx).await {
                        warn!(%channel, error = %e, "notification channel failed");
                    }
                }
                Err(e) => warn!(%channel, error = %e, "notification channel missing"),
            }
        }
        Ok(())
    }
}
