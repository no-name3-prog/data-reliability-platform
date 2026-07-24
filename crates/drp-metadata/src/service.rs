//! Metadata service implementation.

use std::sync::Arc;

use tracing::{info, instrument};

use drp_common::{AssetId, Error, Result, SourceLocation};
use drp_core::{Asset, EventBus, PlatformEvent, PluginContext, PluginRegistry};
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

    /// Discover assets from a connector and upsert them.
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
}
