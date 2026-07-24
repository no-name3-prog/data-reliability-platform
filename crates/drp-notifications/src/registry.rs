//! Built-in notifier registration.

use std::sync::Arc;

use drp_core::PluginRegistry;

use crate::LogNotifier;

/// Register built-in notification channels.
pub fn register_builtin_notifiers(registry: &PluginRegistry) {
    registry.register_notification(Arc::new(LogNotifier::new()));
}
