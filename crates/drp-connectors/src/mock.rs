//! In-memory mock connector for demos and tests.

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};

use drp_common::{AssetKind, DataType, Result, SourceLocation};
use drp_core::{
    Asset, ColumnMeta, ConnectorPlugin, Plugin, PluginCapability, PluginContext, PluginInfo,
};

/// Mock connector that serves a small sample dataset.
pub struct MockConnector {
    info: PluginInfo,
}

impl MockConnector {
    /// Create a new mock connector.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("mock", "Mock Connector", env!("CARGO_PKG_VERSION"))
                .with_description("In-memory sample data for local development and tests")
                .with_capability(PluginCapability::Connector),
        }
    }
}

impl Default for MockConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for MockConnector {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ConnectorPlugin for MockConnector {
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
        let orders = Asset::new(
            "mock.public.orders",
            "orders",
            AssetKind::Table,
            location.clone(),
        )
        .with_columns(vec![
            ColumnMeta::new("order_id", DataType::Integer)
                .required()
                .at(0),
            ColumnMeta::new("customer_email", DataType::String).at(1),
            ColumnMeta::new("amount", DataType::Float).at(2),
            ColumnMeta::new("status", DataType::String).at(3),
        ])
        .with_tag("source", "mock");

        let users = Asset::new(
            "mock.public.users",
            "users",
            AssetKind::Table,
            location.clone(),
        )
        .with_columns(vec![
            ColumnMeta::new("user_id", DataType::Integer)
                .required()
                .at(0),
            ColumnMeta::new("email", DataType::String).at(1),
            ColumnMeta::new("created_at", DataType::Timestamp).at(2),
        ])
        .with_tag("source", "mock");

        Ok(vec![orders, users])
    }

    async fn sample_rows(
        &self,
        asset: &Asset,
        limit: usize,
        _ctx: &PluginContext,
    ) -> Result<Vec<IndexMap<String, Value>>> {
        let rows = match asset.name.as_str() {
            "orders" => vec![
                row(&[
                    ("order_id", json!(1)),
                    ("customer_email", json!("alice@example.com")),
                    ("amount", json!(42.5)),
                    ("status", json!("paid")),
                ]),
                row(&[
                    ("order_id", json!(2)),
                    ("customer_email", json!(null)),
                    ("amount", json!(10.0)),
                    ("status", json!("pending")),
                ]),
                row(&[
                    ("order_id", json!(3)),
                    ("customer_email", json!("bob@example.com")),
                    ("amount", json!(99.9)),
                    ("status", json!("paid")),
                ]),
            ],
            "users" => vec![
                row(&[
                    ("user_id", json!(1)),
                    ("email", json!("alice@example.com")),
                    ("created_at", json!("2024-01-01T00:00:00Z")),
                ]),
                row(&[
                    ("user_id", json!(2)),
                    ("email", json!("bob@example.com")),
                    ("created_at", json!("2024-02-15T12:00:00Z")),
                ]),
            ],
            _ => Vec::new(),
        };
        Ok(rows.into_iter().take(limit).collect())
    }
}

fn row(pairs: &[(&str, Value)]) -> IndexMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}
