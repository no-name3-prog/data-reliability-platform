//! Built-in connector registration helpers.

use std::sync::Arc;

use drp_core::PluginRegistry;

use crate::{FixtureConnector, MockConnector};

/// Register production-ish built-in connectors (mock + empty fixture registry).
pub fn register_builtin_connectors(registry: &PluginRegistry) {
    registry.register_connector(Arc::new(MockConnector::new()));
    registry.register_connector(Arc::new(FixtureConnector::with_sample_data()));
}
