//! Built-in connector registration.

use std::sync::Arc;

use drp_core::PluginRegistry;

use crate::{CsvConnector, FixtureConnector, MockConnector, ParquetConnector, PostgresConnector};

/// Register all built-in connectors.
pub fn register_builtin_connectors(registry: &PluginRegistry) {
    registry.register_connector(Arc::new(MockConnector::new()));
    registry.register_connector(Arc::new(FixtureConnector::with_sample_data()));
    registry.register_connector(Arc::new(PostgresConnector::new()));
    registry.register_connector(Arc::new(CsvConnector::new()));
    registry.register_connector(Arc::new(ParquetConnector::new()));
}
