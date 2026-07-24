//! Example connector plugin — copy this crate when adding a new data source.
//!
//! # How to use this template
//!
//! 1. Copy `plugins/example-connector` → `plugins/my-source` (or a new workspace crate).
//! 2. Rename the package in `Cargo.toml`.
//! 3. Implement real `discover` / `sample_rows` logic.
//! 4. Export `register(registry)`.
//! 5. In `drp-api` composition root, add **one line**:
//!    `drp_plugin_example_connector::register(&platform.plugins);`
//! 6. Do **not** modify `drp-core` or feature services.
//!
//! All development (`make test`, `make lint`) remains containerized.

use std::sync::Arc;

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};

use drp_common::{AssetKind, Result, SourceLocation};
use drp_core::{
    Asset, ConnectorPlugin, Plugin, PluginCapability, PluginContext, PluginInfo, PluginRegistry,
};

/// Example connector plugin id: `example`.
pub struct ExampleConnector {
    info: PluginInfo,
}

impl ExampleConnector {
    /// Create the example connector.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("example", "Example Connector", env!("CARGO_PKG_VERSION"))
                .with_description("Template connector — replace with a real data source")
                .with_capability(PluginCapability::Connector),
        }
    }
}

impl Default for ExampleConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ExampleConnector {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ConnectorPlugin for ExampleConnector {
    async fn test_connection(
        &self,
        _location: &SourceLocation,
        _ctx: &PluginContext,
    ) -> Result<()> {
        Ok(())
    }

    async fn discover(
        &self,
        location: &SourceLocation,
        _ctx: &PluginContext,
    ) -> Result<Vec<Asset>> {
        Ok(vec![Asset::new(
            "example.public.demo",
            "demo",
            AssetKind::Table,
            location.clone(),
        )
        .with_tag("plugin", "example")])
    }

    async fn sample_rows(
        &self,
        _asset: &Asset,
        limit: usize,
        _ctx: &PluginContext,
    ) -> Result<Vec<IndexMap<String, Value>>> {
        let row: IndexMap<String, Value> =
            [("id".into(), json!(1)), ("label".into(), json!("demo"))]
                .into_iter()
                .collect();
        Ok(if limit == 0 { vec![] } else { vec![row] })
    }
}

/// Register this plugin on a registry (call from composition root only).
pub fn register(registry: &PluginRegistry) {
    registry.register_connector(Arc::new(ExampleConnector::new()));
}
