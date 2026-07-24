//! Built-in connector registration helpers.

use std::sync::Arc;

use drp_core::PluginRegistry;

use crate::MockConnector;

/// Register all built-in connectors onto the given registry.
pub fn register_builtin_connectors(registry: &PluginRegistry) {
    registry.register_connector(Arc::new(MockConnector::new()));
}
