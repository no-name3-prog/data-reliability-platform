//! Metadata service implementation.

use std::sync::Arc;

use tracing::{info, instrument};

use drp_common::{AssetId, Error, Result, SourceLocation};
use drp_core::{Asset, CatalogTree, EventBus, PlatformEvent, PluginContext, PluginRegistry};
use drp_storage::Store;

/// Catalog service for assets.
#[derive(Clone)]
pub struct MetadataService {
    store: Arc<dyn Store>,
    plugins: PluginRegistry,
    events: EventBus,
}

impl MetadataService {
    /// Create a metadata service.
    pub fn new(store: Arc<dyn Store>, plugins: PluginRegistry, events: EventBus) -> Self {
        Self {
            store,
            plugins,
            events,
        }
    }

    /// Register or update an asset.
    #[instrument(skip(self, asset), fields(fqn = %asset.fqn))]
    pub async fn upsert_asset(&self, asset: Asset) -> Result<Asset> {
        let saved = self.store.upsert_asset(asset).await?;
        self.events
            .publish(PlatformEvent::AssetUpserted { asset_id: saved.id })
            .await;
        Ok(saved)
    }

    /// Get asset by id.
    pub async fn get_asset(&self, id: &AssetId) -> Result<Asset> {
        self.store
            .get_asset(id)
            .await?
            .ok_or_else(|| Error::not_found(format!("asset {id}")))
    }

    /// Get asset by FQN.
    pub async fn get_asset_by_fqn(&self, fqn: &str) -> Result<Asset> {
        self.store
            .get_asset_by_fqn(fqn)
            .await?
            .ok_or_else(|| Error::not_found(format!("asset fqn={fqn}")))
    }

    /// List assets.
    pub async fn list_assets(&self, limit: Option<usize>) -> Result<Vec<Asset>> {
        self.store.list_assets(limit).await
    }

    /// Delete an asset.
    pub async fn delete_asset(&self, id: &AssetId) -> Result<()> {
        if !self.store.delete_asset(id).await? {
            return Err(Error::not_found(format!("asset {id}")));
        }
        Ok(())
    }

    /// Discover assets from a connector and upsert them into the catalog.
    #[instrument(skip(self, location))]
    pub async fn discover_and_register(
        &self,
        connector_id: &str,
        location: SourceLocation,
    ) -> Result<Vec<Asset>> {
        let connector = self.plugins.connector(connector_id)?;
        let ctx = PluginContext::new();
        connector.test_connection(&location, &ctx).await?;
        let discovered = connector.discover(&location, &ctx).await?;
        info!(
            connector = connector_id,
            count = discovered.len(),
            "discovered assets"
        );

        let mut saved = Vec::with_capacity(discovered.len());
        for asset in discovered {
            saved.push(self.upsert_asset(asset).await?);
        }
        Ok(saved)
    }

    /// Hierarchical catalog discovery + persist every table as an asset (with columns).
    #[instrument(skip(self, location))]
    pub async fn discover_catalog_and_register(
        &self,
        connector_id: &str,
        location: SourceLocation,
    ) -> Result<(CatalogTree, Vec<Asset>)> {
        let connector = self.plugins.connector(connector_id)?;
        let ctx = PluginContext::new();
        connector.test_connection(&location, &ctx).await?;
        let tree = connector.discover_catalog(&location, &ctx).await?;

        let mut saved = Vec::new();
        for t in tree.all_tables() {
            let mut asset = Asset::new(t.fqn.clone(), t.name.clone(), t.kind, location.clone())
                .with_columns(t.columns.clone());
            for (k, v) in &t.properties {
                asset = asset.with_tag(k, v);
            }
            // Preserve hierarchy for sampling
            if let Some(schema) = t.properties.get("schema") {
                asset = asset.with_tag("schema", schema);
            }
            if let Some(path) = t.properties.get("path") {
                asset = asset.with_tag("path", path);
            }
            saved.push(self.upsert_asset(asset).await?);
        }

        info!(
            connector = connector_id,
            databases = tree.databases.len(),
            tables = saved.len(),
            "catalog discovered and stored"
        );
        Ok((tree, saved))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drp_common::{AssetKind, SourceLocation};
    use drp_connectors::register_builtin_connectors;
    use drp_storage::MemoryStore;

    #[tokio::test]
    async fn discover_mock_assets() {
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let plugins = PluginRegistry::new();
        register_builtin_connectors(&plugins);
        let svc = MetadataService::new(store, plugins, EventBus::new());
        let assets = svc
            .discover_and_register("mock", SourceLocation::new("mock", "mock://local"))
            .await
            .unwrap();
        assert_eq!(assets.len(), 2);
        assert!(assets.iter().any(|a| a.kind == AssetKind::Table));
    }

    #[tokio::test]
    async fn discover_csv_catalog() {
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let plugins = PluginRegistry::new();
        register_builtin_connectors(&plugins);
        let svc = MetadataService::new(store, plugins, EventBus::new());
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../drp-connectors/testdata");
        let (tree, assets) = svc
            .discover_catalog_and_register("csv", SourceLocation::new("csv", path))
            .await
            .unwrap();
        assert!(tree.table_count() >= 1);
        assert!(!assets.is_empty());
        assert!(assets[0].columns.iter().any(|c| c.name == "order_id"));
    }
}
