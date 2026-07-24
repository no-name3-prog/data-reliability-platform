//! In-memory mock connectors for demos, unit tests, and integration tests.
//!
//! - [`MockConnector`] — small fixed sample dataset (`orders`, `users`)
//! - [`FixtureConnector`] — configurable tables/rows for regression fixtures
//! - [`FailingConnector`] — always fails (negative-path tests)

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use indexmap::IndexMap;
use parking_lot::RwLock;
use serde_json::{json, Value};

use drp_common::{AssetKind, DataType, Error, Result, SourceLocation};
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
        Ok(vec![
            sample_orders_asset(location.clone()),
            sample_users_asset(location.clone()),
        ])
    }

    async fn sample_rows(
        &self,
        asset: &Asset,
        limit: usize,
        _ctx: &PluginContext,
    ) -> Result<Vec<IndexMap<String, Value>>> {
        let rows = match asset.name.as_str() {
            "orders" => sample_orders_rows(),
            "users" => sample_users_rows(),
            _ => Vec::new(),
        };
        Ok(rows.into_iter().take(limit).collect())
    }
}

/// A single logical table served by [`FixtureConnector`].
#[derive(Debug, Clone)]
pub struct FixtureTable {
    /// Asset FQN (e.g. `fixture.public.orders`).
    pub fqn: String,
    /// Table / asset name.
    pub name: String,
    /// Columns.
    pub columns: Vec<ColumnMeta>,
    /// Sample rows as JSON objects.
    pub rows: Vec<IndexMap<String, Value>>,
}

impl FixtureTable {
    /// Construct a fixture table.
    pub fn new(fqn: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            fqn: fqn.into(),
            name: name.into(),
            columns: Vec::new(),
            rows: Vec::new(),
        }
    }

    /// Attach columns.
    pub fn with_columns(mut self, columns: Vec<ColumnMeta>) -> Self {
        self.columns = columns;
        self
    }

    /// Attach rows.
    pub fn with_rows(mut self, rows: Vec<IndexMap<String, Value>>) -> Self {
        self.rows = rows;
        self
    }
}

/// Configurable connector for integration / regression fixtures.
pub struct FixtureConnector {
    info: PluginInfo,
    tables: RwLock<HashMap<String, FixtureTable>>,
}

impl FixtureConnector {
    /// Create an empty fixture connector (plugin id `fixture`).
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("fixture", "Fixture Connector", env!("CARGO_PKG_VERSION"))
                .with_description("Configurable in-memory tables for tests")
                .with_capability(PluginCapability::Connector),
            tables: RwLock::new(HashMap::new()),
        }
    }

    /// Seed with the standard orders/users sample (same shape as mock).
    pub fn with_sample_data() -> Self {
        let c = Self::new();
        c.upsert_table(
            FixtureTable::new("fixture.public.orders", "orders")
                .with_columns(sample_orders_columns())
                .with_rows(sample_orders_rows()),
        );
        c.upsert_table(
            FixtureTable::new("fixture.public.users", "users")
                .with_columns(sample_users_columns())
                .with_rows(sample_users_rows()),
        );
        c
    }

    /// Insert or replace a table.
    pub fn upsert_table(&self, table: FixtureTable) {
        self.tables.write().insert(table.name.clone(), table);
    }

    /// Number of tables.
    pub fn table_count(&self) -> usize {
        self.tables.read().len()
    }
}

impl Default for FixtureConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for FixtureConnector {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ConnectorPlugin for FixtureConnector {
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
        let tables = self.tables.read();
        let mut assets = Vec::with_capacity(tables.len());
        for t in tables.values() {
            assets.push(
                Asset::new(
                    t.fqn.clone(),
                    t.name.clone(),
                    AssetKind::Table,
                    location.clone(),
                )
                .with_columns(t.columns.clone())
                .with_tag("source", "fixture"),
            );
        }
        assets.sort_by(|a, b| a.fqn.cmp(&b.fqn));
        Ok(assets)
    }

    async fn sample_rows(
        &self,
        asset: &Asset,
        limit: usize,
        _ctx: &PluginContext,
    ) -> Result<Vec<IndexMap<String, Value>>> {
        let tables = self.tables.read();
        let Some(table) = tables.get(&asset.name) else {
            return Ok(vec![]);
        };
        Ok(table.rows.iter().take(limit).cloned().collect())
    }
}

/// Connector that always fails — useful for negative-path tests.
pub struct FailingConnector {
    info: PluginInfo,
    message: String,
}

impl FailingConnector {
    /// Create a failing connector with a custom message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            info: PluginInfo::new("failing", "Failing Connector", env!("CARGO_PKG_VERSION"))
                .with_description("Always fails — for negative tests")
                .with_capability(PluginCapability::Connector),
            message: message.into(),
        }
    }
}

impl Default for FailingConnector {
    fn default() -> Self {
        Self::new("simulated connector failure")
    }
}

impl Plugin for FailingConnector {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ConnectorPlugin for FailingConnector {
    async fn test_connection(
        &self,
        _location: &SourceLocation,
        _ctx: &PluginContext,
    ) -> Result<()> {
        Err(Error::connector(self.message.clone()))
    }

    async fn discover(
        &self,
        _location: &SourceLocation,
        _ctx: &PluginContext,
    ) -> Result<Vec<Asset>> {
        Err(Error::connector(self.message.clone()))
    }

    async fn sample_rows(
        &self,
        _asset: &Asset,
        _limit: usize,
        _ctx: &PluginContext,
    ) -> Result<Vec<IndexMap<String, Value>>> {
        Err(Error::connector(self.message.clone()))
    }
}

/// Shared sample helpers (used by mock + fixture seed).
pub fn sample_orders_columns() -> Vec<ColumnMeta> {
    vec![
        ColumnMeta::new("order_id", DataType::Integer)
            .required()
            .at(0),
        ColumnMeta::new("customer_email", DataType::String).at(1),
        ColumnMeta::new("amount", DataType::Float).at(2),
        ColumnMeta::new("status", DataType::String).at(3),
    ]
}

/// Sample users columns.
pub fn sample_users_columns() -> Vec<ColumnMeta> {
    vec![
        ColumnMeta::new("user_id", DataType::Integer)
            .required()
            .at(0),
        ColumnMeta::new("email", DataType::String).at(1),
        ColumnMeta::new("created_at", DataType::Timestamp).at(2),
    ]
}

/// Sample orders asset.
pub fn sample_orders_asset(location: SourceLocation) -> Asset {
    Asset::new("mock.public.orders", "orders", AssetKind::Table, location)
        .with_columns(sample_orders_columns())
        .with_tag("source", "mock")
}

/// Sample users asset.
pub fn sample_users_asset(location: SourceLocation) -> Asset {
    Asset::new("mock.public.users", "users", AssetKind::Table, location)
        .with_columns(sample_users_columns())
        .with_tag("source", "mock")
}

/// Sample orders rows (includes one null email for DQ tests).
pub fn sample_orders_rows() -> Vec<IndexMap<String, Value>> {
    vec![
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
        row(&[
            ("order_id", json!(4)),
            ("customer_email", json!("carol@example.com")),
            ("amount", json!(5.25)),
            ("status", json!("cancelled")),
        ]),
    ]
}

/// Sample users rows.
pub fn sample_users_rows() -> Vec<IndexMap<String, Value>> {
    vec![
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
    ]
}

/// Build a row map.
pub fn row(pairs: &[(&str, Value)]) -> IndexMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

/// Arc helper for registration.
pub type SharedFixtureConnector = Arc<FixtureConnector>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_discovers_two_tables() {
        let c = MockConnector::new();
        let assets = c
            .discover(
                &SourceLocation::new("mock", "mock://"),
                &PluginContext::new(),
            )
            .await
            .unwrap();
        assert_eq!(assets.len(), 2);
    }

    #[tokio::test]
    async fn mock_orders_include_null_email() {
        let c = MockConnector::new();
        let asset = sample_orders_asset(SourceLocation::new("mock", "mock://"));
        let rows = c
            .sample_rows(&asset, 100, &PluginContext::new())
            .await
            .unwrap();
        assert!(rows
            .iter()
            .any(|r| r.get("customer_email") == Some(&Value::Null)));
    }

    #[tokio::test]
    async fn fixture_connector_is_configurable() {
        let c = FixtureConnector::new();
        c.upsert_table(
            FixtureTable::new("t.public.a", "a").with_rows(vec![row(&[("id", json!(1))])]),
        );
        assert_eq!(c.table_count(), 1);
        let assets = c
            .discover(
                &SourceLocation::new("fixture", "f://"),
                &PluginContext::new(),
            )
            .await
            .unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "a");
    }

    #[tokio::test]
    async fn failing_connector_errors() {
        let c = FailingConnector::new("boom");
        let err = c
            .test_connection(&SourceLocation::new("x", "y"), &PluginContext::new())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "connector_error");
    }
}
