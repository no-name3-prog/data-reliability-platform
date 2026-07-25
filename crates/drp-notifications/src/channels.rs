//! Built-in notification channel plugins: log, Slack, email, webhook.

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};
use tracing::{info, warn};

use drp_common::{Error, Result};
use drp_core::{NotificationPlugin, Plugin, PluginCapability, PluginContext, PluginInfo};

/// Logs notifications via `tracing` (always available).
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
        info!(%subject, %body, ?metadata, channel = "log", "notification");
        Ok(())
    }
}

/// Slack incoming webhook notifier.
pub struct SlackNotifier {
    info: PluginInfo,
    webhook_url: String,
    client: reqwest::Client,
}

impl SlackNotifier {
    /// Create with optional webhook URL (empty ⇒ dry-run log).
    pub fn new(webhook_url: impl Into<String>) -> Self {
        Self {
            info: PluginInfo::new("slack", "Slack Notifier", env!("CARGO_PKG_VERSION"))
                .with_description("Posts alerts to a Slack incoming webhook")
                .with_capability(PluginCapability::Notification),
            webhook_url: webhook_url.into(),
            client: reqwest::Client::new(),
        }
    }
}

impl Plugin for SlackNotifier {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl NotificationPlugin for SlackNotifier {
    async fn send(
        &self,
        subject: &str,
        body: &str,
        metadata: &IndexMap<String, Value>,
        _ctx: &PluginContext,
    ) -> Result<()> {
        let text = format!("*{subject}*\n{body}");
        if self.webhook_url.is_empty() {
            info!(%subject, %body, ?metadata, channel = "slack", "slack dry-run (no webhook_url)");
            return Ok(());
        }
        let payload = json!({
            "text": text,
            "blocks": [
                {
                    "type": "header",
                    "text": { "type": "plain_text", "text": subject }
                },
                {
                    "type": "section",
                    "text": { "type": "mrkdwn", "text": body }
                }
            ]
        });
        let resp = self
            .client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::notification(format!("slack request failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(Error::notification(format!(
                "slack webhook HTTP {status}: {t}"
            )));
        }
        Ok(())
    }
}

/// Email notifier (HTTP email bridge or dry-run log).
pub struct EmailNotifier {
    info: PluginInfo,
    to: String,
    webhook_url: String,
    client: reqwest::Client,
}

impl EmailNotifier {
    /// Create email notifier.
    pub fn new(to: impl Into<String>, webhook_url: impl Into<String>) -> Self {
        Self {
            info: PluginInfo::new("email", "Email Notifier", env!("CARGO_PKG_VERSION"))
                .with_description("Sends email via HTTP bridge webhook or dry-run log")
                .with_capability(PluginCapability::Notification),
            to: to.into(),
            webhook_url: webhook_url.into(),
            client: reqwest::Client::new(),
        }
    }
}

impl Plugin for EmailNotifier {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl NotificationPlugin for EmailNotifier {
    async fn send(
        &self,
        subject: &str,
        body: &str,
        metadata: &IndexMap<String, Value>,
        _ctx: &PluginContext,
    ) -> Result<()> {
        if self.webhook_url.is_empty() {
            info!(
                %subject,
                %body,
                to = %self.to,
                ?metadata,
                channel = "email",
                "email dry-run (no email_webhook_url)"
            );
            return Ok(());
        }
        let payload = json!({
            "to": self.to,
            "subject": subject,
            "body": body,
            "metadata": metadata,
        });
        let resp = self
            .client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::notification(format!("email request failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(Error::notification(format!(
                "email webhook HTTP {status}: {t}"
            )));
        }
        Ok(())
    }
}

/// Generic HTTPS webhook notifier (JSON POST).
pub struct WebhookNotifier {
    info: PluginInfo,
    url: String,
    client: reqwest::Client,
}

impl WebhookNotifier {
    /// Create webhook notifier.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            info: PluginInfo::new("webhook", "Webhook Notifier", env!("CARGO_PKG_VERSION"))
                .with_description("POSTs JSON incident payloads to a webhook URL")
                .with_capability(PluginCapability::Notification),
            url: url.into(),
            client: reqwest::Client::new(),
        }
    }
}

impl Plugin for WebhookNotifier {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl NotificationPlugin for WebhookNotifier {
    async fn send(
        &self,
        subject: &str,
        body: &str,
        metadata: &IndexMap<String, Value>,
        _ctx: &PluginContext,
    ) -> Result<()> {
        if self.url.is_empty() {
            info!(%subject, %body, ?metadata, channel = "webhook", "webhook dry-run (no webhook_url)");
            return Ok(());
        }
        let payload = json!({
            "subject": subject,
            "body": body,
            "metadata": metadata,
            "source": "data-reliability-platform",
        });
        let resp = self
            .client
            .post(&self.url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::notification(format!("webhook request failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let t = resp.text().await.unwrap_or_default();
            warn!(%status, body = %t, "webhook non-success");
            return Err(Error::notification(format!("webhook HTTP {status}: {t}")));
        }
        Ok(())
    }
}
