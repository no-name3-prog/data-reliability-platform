//! Built-in notifier registration.

use std::sync::Arc;

use drp_common::NotificationsConfig;
use drp_core::PluginRegistry;

use crate::{EmailNotifier, LogNotifier, SlackNotifier, WebhookNotifier};

/// Register built-in notification channels (log, slack, email, webhook).
pub fn register_builtin_notifiers(registry: &PluginRegistry, cfg: &NotificationsConfig) {
    registry.register_notification(Arc::new(LogNotifier::new()));
    registry.register_notification(Arc::new(SlackNotifier::new(cfg.slack_webhook_url.clone())));
    registry.register_notification(Arc::new(EmailNotifier::new(
        cfg.email_to.clone(),
        cfg.email_webhook_url.clone(),
    )));
    registry.register_notification(Arc::new(WebhookNotifier::new(cfg.webhook_url.clone())));
}
